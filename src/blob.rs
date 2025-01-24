//! # Blob directory management.

use core::cmp::max;
use std::ffi::OsStr;
use std::io::{Cursor, Seek};
use std::iter::FusedIterator;
use std::mem;
use std::path::{Path, PathBuf};

use anyhow::{ensure, format_err, Context as _, Result};
use base64::Engine as _;
use futures::StreamExt;
use image::codecs::jpeg::JpegEncoder;
use image::ImageReader;
use image::{DynamicImage, GenericImage, GenericImageView, ImageFormat, Pixel, Rgba};
use num_traits::FromPrimitive;
use tokio::io::AsyncWriteExt;
use tokio::{fs, io, task};
use tokio_stream::wrappers::ReadDirStream;

use crate::config::Config;
use crate::constants::{self, MediaQuality};
use crate::context::Context;
use crate::events::EventType;
use crate::log::LogExt;

/// Represents a file in the blob directory.
///
/// The object has a name, which will always be valid UTF-8.  Having a
/// blob object does not imply the respective file exists, however
/// when using one of the `create*()` methods a unique file is
/// created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobObject<'a> {
    blobdir: &'a Path,

    /// The name of the file on the disc.
    /// Note that this is NOT the user-visible filename,
    /// which is only stored in Param::Filename on the message.
    name: String,
}

#[derive(Debug, Clone)]
enum ImageOutputFormat {
    Png,
    Jpeg { quality: u8 },
}

impl<'a> BlobObject<'a> {
    /// Creates a new file, returning a tuple of the name and the handle.
    async fn create_new_file(
        context: &Context,
        dir: &Path,
        stem: &str,
        ext: &str,
    ) -> Result<(String, fs::File)> {
        const MAX_ATTEMPT: u32 = 16;
        let mut attempt = 0;
        let mut name = format!("{stem}{ext}");
        loop {
            attempt += 1;
            let path = dir.join(&name);
            match fs::OpenOptions::new()
                // Using `create_new(true)` in order to avoid race conditions
                // when creating multiple files with the same name.
                .create_new(true)
                .write(true)
                .open(&path)
                .await
            {
                Ok(file) => return Ok((name, file)),
                Err(err) => {
                    if attempt >= MAX_ATTEMPT {
                        return Err(err).context("failed to create file");
                    } else if attempt == 1 && !dir.exists() {
                        fs::create_dir_all(dir).await.log_err(context).ok();
                    } else {
                        name = format!("{}-{}{}", stem, rand::random::<u32>(), ext);
                    }
                }
            }
        }
    }

    /// Creates a new blob object with unique name by copying an existing file.
    ///
    /// This creates a new blob
    /// and copies an existing file into it.  This is done in a
    /// in way which avoids race-conditions when multiple files are
    /// concurrently created.
    pub async fn create_and_copy(context: &'a Context, src: &Path) -> Result<BlobObject<'a>> {
        let mut src_file = fs::File::open(src)
            .await
            .with_context(|| format!("failed to open file {}", src.display()))?;
        let (stem, ext) = BlobObject::sanitise_name(&src.to_string_lossy());
        let (name, mut dst_file) =
            BlobObject::create_new_file(context, context.get_blobdir(), &stem, &ext).await?;
        let name_for_err = name.clone();
        if let Err(err) = io::copy(&mut src_file, &mut dst_file).await {
            // Attempt to remove the failed file, swallow errors resulting from that.
            let path = context.get_blobdir().join(&name_for_err);
            fs::remove_file(path).await.ok();
            return Err(err).context("failed to copy file");
        }

        // Ensure that all buffered bytes are written
        dst_file.flush().await?;

        let blob = BlobObject {
            blobdir: context.get_blobdir(),
            name: format!("$BLOBDIR/{name}"),
        };
        context.emit_event(EventType::NewBlobFile(blob.as_name().to_string()));
        Ok(blob)
    }

    /// Creates a blob object by copying or renaming an existing file.
    /// If the source file is already in the blobdir, it will be renamed,
    /// otherwise it will be copied to the blobdir first.
    ///
    /// In order to deduplicate files that contain the same data,
    /// the file will be named `<hash>.<extension>`, e.g. `ce940175885d7b78f7b7e9f1396611f.jpg`.
    /// The `original_name` param is only used to get the extension.
    ///
    /// This is done in a in way which avoids race-conditions when multiple files are
    /// concurrently created.
    pub fn create_and_deduplicate(
        context: &'a Context,
        src: &Path,
        original_name: &Path,
    ) -> Result<BlobObject<'a>> {
        // `create_and_deduplicate{_from_bytes}()` do blocking I/O, but can still be called
        // from an async context thanks to `block_in_place()`.
        // Tokio's "async" I/O functions are also just thin wrappers around the blocking I/O syscalls,
        // so we are doing essentially the same here.
        task::block_in_place(|| {
            let temp_path;
            let src_in_blobdir: &Path;
            let blobdir = context.get_blobdir();

            if src.starts_with(blobdir) {
                src_in_blobdir = src;
            } else {
                info!(
                    context,
                    "Source file not in blobdir. Copying instead of moving in order to prevent moving a file that was still needed."
                );
                temp_path = blobdir.join(format!("tmp-{}", rand::random::<u64>()));
                if std::fs::copy(src, &temp_path).is_err() {
                    // Maybe the blobdir didn't exist
                    std::fs::create_dir_all(blobdir).log_err(context).ok();
                    std::fs::copy(src, &temp_path).context("Copying new blobfile failed")?;
                };
                src_in_blobdir = &temp_path;
            }

            let hash = file_hash(src_in_blobdir)?.to_hex();
            let hash = hash.as_str();
            let hash = hash.get(0..31).unwrap_or(hash);
            let new_file =
                if let Some(extension) = original_name.extension().filter(|e| e.len() <= 32) {
                    format!(
                        "$BLOBDIR/{hash}.{}",
                        extension.to_string_lossy().to_lowercase()
                    )
                } else {
                    format!("$BLOBDIR/{hash}")
                };

            let blob = BlobObject {
                blobdir,
                name: new_file,
            };
            let new_path = blob.to_abs_path();

            // This will also replace an already-existing file.
            // Renaming is atomic, so this will avoid race conditions.
            std::fs::rename(src_in_blobdir, &new_path)?;

            context.emit_event(EventType::NewBlobFile(blob.as_name().to_string()));
            Ok(blob)
        })
    }

    /// Creates a new blob object with the file contents in `data`.
    /// In order to deduplicate files that contain the same data,
    /// the file will be named `<hash>.<extension>`, e.g. `ce940175885d7b78f7b7e9f1396611f.jpg`.
    /// The `original_name` param is only used to get the extension.
    ///
    /// The `data` will be written into the file without race-conditions.
    ///
    /// This function does blocking I/O, but it can still be called from an async context
    /// because `block_in_place()` is used to leave the async runtime if necessary.
    pub fn create_and_deduplicate_from_bytes(
        context: &'a Context,
        data: &[u8],
        original_name: &str,
    ) -> Result<BlobObject<'a>> {
        task::block_in_place(|| {
            let blobdir = context.get_blobdir();
            let temp_path = blobdir.join(format!("tmp-{}", rand::random::<u64>()));
            if std::fs::write(&temp_path, data).is_err() {
                // Maybe the blobdir didn't exist
                std::fs::create_dir_all(blobdir).log_err(context).ok();
                std::fs::write(&temp_path, data).context("writing new blobfile failed")?;
            };

            BlobObject::create_and_deduplicate(context, &temp_path, Path::new(original_name))
        })
    }

    /// Creates a blob from a file, possibly copying it to the blobdir.
    ///
    /// If the source file is not a path to into the blob directory
    /// the file will be copied into the blob directory first.  If the
    /// source file is already in the blobdir it will not be copied
    /// and only be created if it is a valid blobname, that is no
    /// subdirectory is used and [BlobObject::sanitise_name] does not
    /// modify the filename.
    ///
    /// Paths into the blob directory may be either defined by an absolute path
    /// or by the relative prefix `$BLOBDIR`.
    pub async fn new_from_path(context: &'a Context, src: &Path) -> Result<BlobObject<'a>> {
        if src.starts_with(context.get_blobdir()) {
            BlobObject::from_path(context, src)
        } else if src.starts_with("$BLOBDIR/") {
            BlobObject::from_name(context, src.to_str().unwrap_or_default().to_string())
        } else {
            BlobObject::create_and_copy(context, src).await
        }
    }

    /// Returns a [BlobObject] for an existing blob from a path.
    ///
    /// The path must designate a file directly in the blobdir and
    /// must use a valid blob name.  That is after sanitisation the
    /// name must still be the same, that means it must be valid UTF-8
    /// and not have any special characters in it.
    pub fn from_path(context: &'a Context, path: &Path) -> Result<BlobObject<'a>> {
        let rel_path = path
            .strip_prefix(context.get_blobdir())
            .with_context(|| format!("wrong blobdir: {}", path.display()))?;
        if !BlobObject::is_acceptible_blob_name(rel_path) {
            return Err(format_err!("bad blob name: {}", rel_path.display()));
        }
        let name = rel_path.to_str().context("wrong name")?;
        BlobObject::from_name(context, name.to_string())
    }

    /// Returns a [BlobObject] for an existing blob.
    ///
    /// The `name` may optionally be prefixed with the `$BLOBDIR/`
    /// prefixed, as returned by [BlobObject::as_name].  This is how
    /// you want to create a [BlobObject] for a filename read from the
    /// database.
    pub fn from_name(context: &'a Context, name: String) -> Result<BlobObject<'a>> {
        let name: String = match name.starts_with("$BLOBDIR/") {
            true => name.splitn(2, '/').last().unwrap().to_string(),
            false => name,
        };
        if !BlobObject::is_acceptible_blob_name(&name) {
            return Err(format_err!("not an acceptable blob name: {}", &name));
        }
        Ok(BlobObject {
            blobdir: context.get_blobdir(),
            name: format!("$BLOBDIR/{name}"),
        })
    }

    /// Returns the absolute path to the blob in the filesystem.
    pub fn to_abs_path(&self) -> PathBuf {
        let fname = Path::new(&self.name).strip_prefix("$BLOBDIR/").unwrap();
        self.blobdir.join(fname)
    }

    /// Returns the blob name, as stored in the database.
    ///
    /// This returns the blob in the `$BLOBDIR/<name>` format used in
    /// the database.  Do not use this unless you're about to store
    /// this string in the database or [Params].  Eventually even
    /// those conversions should be handled by the type system.
    ///
    /// Note that this is NOT the user-visible filename,
    /// which is only stored in Param::Filename on the message.
    ///
    /// [Params]: crate::param::Params
    pub fn as_name(&self) -> &str {
        &self.name
    }

    /// Returns the filename of the blob.
    pub fn as_file_name(&self) -> &str {
        self.name.rsplit('/').next().unwrap_or_default()
    }

    /// The path relative in the blob directory.
    pub fn as_rel_path(&self) -> &Path {
        Path::new(self.as_file_name())
    }

    /// Returns the extension of the blob.
    ///
    /// If a blob's filename has an extension, it is always guaranteed
    /// to be lowercase.
    pub fn suffix(&self) -> Option<&str> {
        let ext = self.name.rsplit('.').next();
        if ext == Some(&self.name) {
            None
        } else {
            ext
        }
    }

    /// Create a safe name based on a messy input string.
    ///
    /// The safe name will be a valid filename on Unix and Windows and
    /// not contain any path separators.  The input can contain path
    /// segments separated by either Unix or Windows path separators,
    /// the rightmost non-empty segment will be used as name,
    /// sanitised for special characters.
    ///
    /// The resulting name is returned as a tuple, the first part
    /// being the stem or basename and the second being an extension,
    /// including the dot.  E.g. "foo.txt" is returned as `("foo",
    /// ".txt")` while "bar" is returned as `("bar", "")`.
    ///
    /// The extension part will always be lowercased.
    fn sanitise_name(name: &str) -> (String, String) {
        let mut name = name;
        for part in name.rsplit('/') {
            if !part.is_empty() {
                name = part;
                break;
            }
        }
        for part in name.rsplit('\\') {
            if !part.is_empty() {
                name = part;
                break;
            }
        }
        let opts = sanitize_filename::Options {
            truncate: true,
            windows: true,
            replacement: "",
        };

        let name = sanitize_filename::sanitize_with_options(name, opts);
        // Let's take a tricky filename,
        // "file.with_lots_of_characters_behind_point_and_double_ending.tar.gz" as an example.
        // Assume that the extension is 32 chars maximum.
        let ext: String = name
            .chars()
            .rev()
            .take_while(|c| {
                (!c.is_ascii_punctuation() || *c == '.') && !c.is_whitespace() && !c.is_control()
            })
            .take(33)
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .collect();
        // ext == "nd_point_and_double_ending.tar.gz"

        // Split it into "nd_point_and_double_ending" and "tar.gz":
        let mut iter = ext.splitn(2, '.');
        iter.next();

        let ext = iter.next().unwrap_or_default();
        let ext = if ext.is_empty() {
            String::new()
        } else {
            format!(".{ext}")
            // ".tar.gz"
        };
        let stem = name
            .strip_suffix(&ext)
            .unwrap_or_default()
            .chars()
            .take(64)
            .collect();
        (stem, ext.to_lowercase())
    }

    /// Checks whether a name is a valid blob name.
    ///
    /// This is slightly less strict than stanitise_name, presumably
    /// someone already created a file with such a name so we just
    /// ensure it's not actually a path in disguise is actually utf-8.
    fn is_acceptible_blob_name(name: impl AsRef<OsStr>) -> bool {
        let uname = match name.as_ref().to_str() {
            Some(name) => name,
            None => return false,
        };
        if uname.find('/').is_some() {
            return false;
        }
        if uname.find('\\').is_some() {
            return false;
        }
        if uname.find('\0').is_some() {
            return false;
        }
        true
    }

    /// Returns path to the stored Base64-decoded blob.
    ///
    /// If `data` represents an image of known format, this adds the corresponding extension.
    ///
    /// Even though this function is not async, it's OK to call it from an async context.
    pub(crate) fn store_from_base64(context: &Context, data: &str) -> Result<String> {
        let buf = base64::engine::general_purpose::STANDARD.decode(data)?;
        let name = if let Ok(format) = image::guess_format(&buf) {
            if let Some(ext) = format.extensions_str().first() {
                format!("file.{ext}")
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let blob = BlobObject::create_and_deduplicate_from_bytes(context, &buf, &name)?;
        Ok(blob.as_name().to_string())
    }

    pub async fn recode_to_avatar_size(&mut self, context: &Context) -> Result<()> {
        let img_wh =
            match MediaQuality::from_i32(context.get_config_int(Config::MediaQuality).await?)
                .unwrap_or_default()
            {
                MediaQuality::Balanced => constants::BALANCED_AVATAR_SIZE,
                MediaQuality::Worse => constants::WORSE_AVATAR_SIZE,
            };

        let maybe_sticker = &mut false;
        let strict_limits = true;
        // max_bytes is 20_000 bytes: Outlook servers don't allow headers larger than 32k.
        // 32 / 4 * 3 = 24k if you account for base64 encoding. To be safe, we reduced this to 20k.
        self.recode_to_size(
            context,
            None, // The name of an avatar doesn't matter
            maybe_sticker,
            img_wh,
            20_000,
            strict_limits,
        )?;

        Ok(())
    }

    /// Recodes an image pointed by a [BlobObject] so that it fits into limits on the image width,
    /// height and file size specified by the config.
    ///
    /// On some platforms images are passed to the core as [`crate::message::Viewtype::Sticker`] in
    /// which case `maybe_sticker` flag should be set. We recheck if an image is a true sticker
    /// assuming that it must have at least one fully transparent corner, otherwise this flag is
    /// reset.
    pub async fn recode_to_image_size(
        &mut self,
        context: &Context,
        name: Option<String>,
        maybe_sticker: &mut bool,
    ) -> Result<String> {
        let (img_wh, max_bytes) =
            match MediaQuality::from_i32(context.get_config_int(Config::MediaQuality).await?)
                .unwrap_or_default()
            {
                MediaQuality::Balanced => (
                    constants::BALANCED_IMAGE_SIZE,
                    constants::BALANCED_IMAGE_BYTES,
                ),
                MediaQuality::Worse => (constants::WORSE_IMAGE_SIZE, constants::WORSE_IMAGE_BYTES),
            };
        let strict_limits = false;
        let new_name = self.recode_to_size(
            context,
            name,
            maybe_sticker,
            img_wh,
            max_bytes,
            strict_limits,
        )?;

        Ok(new_name)
    }

    /// If `!strict_limits`, then if `max_bytes` is exceeded, reduce the image to `img_wh` and just
    /// proceed with the result.
    ///
    /// This modifies the blob object in-place.
    ///
    /// Additionally, if you pass the user-visible filename as `name`
    /// then the updated user-visible filename will be returned;
    /// this may be necessary because the format may be changed to JPG,
    /// i.e. "image.png" -> "image.jpg".
    fn recode_to_size(
        &mut self,
        context: &Context,
        name: Option<String>,
        maybe_sticker: &mut bool,
        mut img_wh: u32,
        max_bytes: usize,
        strict_limits: bool,
    ) -> Result<String> {
        // Add white background only to avatars to spare the CPU.
        let mut add_white_bg = img_wh <= constants::BALANCED_AVATAR_SIZE;
        let mut no_exif = false;
        let no_exif_ref = &mut no_exif;
        let mut name = name.unwrap_or_else(|| self.name.clone());
        let original_name = name.clone();
        let res: Result<String> = tokio::task::block_in_place(move || {
            let mut file = std::fs::File::open(self.to_abs_path())?;
            let (nr_bytes, exif) = image_metadata(&file)?;
            *no_exif_ref = exif.is_none();
            // It's strange that BufReader modifies a file position while it takes a non-mut
            // reference. Ok, just rewind it.
            file.rewind()?;
            let imgreader = ImageReader::new(std::io::BufReader::new(&file)).with_guessed_format();
            let imgreader = match imgreader {
                Ok(ir) => ir,
                _ => {
                    file.rewind()?;
                    ImageReader::with_format(
                        std::io::BufReader::new(&file),
                        ImageFormat::from_path(self.to_abs_path())?,
                    )
                }
            };
            let fmt = imgreader.format().context("No format??")?;
            let mut img = imgreader.decode().context("image decode failure")?;
            let orientation = exif.as_ref().map(|exif| exif_orientation(exif, context));
            let mut encoded = Vec::new();

            if *maybe_sticker {
                let x_max = img.width().saturating_sub(1);
                let y_max = img.height().saturating_sub(1);
                *maybe_sticker = img.in_bounds(x_max, y_max)
                    && (img.get_pixel(0, 0).0[3] == 0
                        || img.get_pixel(x_max, 0).0[3] == 0
                        || img.get_pixel(0, y_max).0[3] == 0
                        || img.get_pixel(x_max, y_max).0[3] == 0);
            }
            if *maybe_sticker && exif.is_none() {
                return Ok(name);
            }

            img = match orientation {
                Some(90) => img.rotate90(),
                Some(180) => img.rotate180(),
                Some(270) => img.rotate270(),
                _ => img,
            };

            let exceeds_wh = img.width() > img_wh || img.height() > img_wh;
            let exceeds_max_bytes = nr_bytes > max_bytes as u64;

            let jpeg_quality = 75;
            let ofmt = match fmt {
                ImageFormat::Png if !exceeds_max_bytes => ImageOutputFormat::Png,
                ImageFormat::Jpeg => {
                    add_white_bg = false;
                    ImageOutputFormat::Jpeg {
                        quality: jpeg_quality,
                    }
                }
                _ => ImageOutputFormat::Jpeg {
                    quality: jpeg_quality,
                },
            };
            // We need to rewrite images with Exif to remove metadata such as location,
            // camera model, etc.
            //
            // TODO: Fix lost animation and transparency when recoding using the `image` crate. And
            // also `Viewtype::Gif` (maybe renamed to `Animation`) should be used for animated
            // images.
            let do_scale = exceeds_max_bytes
                || strict_limits
                    && (exceeds_wh
                        || exif.is_some() && {
                            if mem::take(&mut add_white_bg) {
                                self::add_white_bg(&mut img);
                            }
                            encoded_img_exceeds_bytes(
                                context,
                                &img,
                                ofmt.clone(),
                                max_bytes,
                                &mut encoded,
                            )?
                        });

            if do_scale {
                if !exceeds_wh {
                    img_wh = max(img.width(), img.height());
                    // PNGs and WebPs may be huge because of animation, which is lost by the `image`
                    // crate when recoding, so don't scale them down.
                    if matches!(fmt, ImageFormat::Jpeg) || !encoded.is_empty() {
                        img_wh = img_wh * 2 / 3;
                    }
                }

                loop {
                    if mem::take(&mut add_white_bg) {
                        self::add_white_bg(&mut img);
                    }
                    let new_img = img.thumbnail(img_wh, img_wh);

                    if encoded_img_exceeds_bytes(
                        context,
                        &new_img,
                        ofmt.clone(),
                        max_bytes,
                        &mut encoded,
                    )? && strict_limits
                    {
                        if img_wh < 20 {
                            return Err(format_err!(
                                "Failed to scale image to below {}B.",
                                max_bytes,
                            ));
                        }

                        img_wh = img_wh * 2 / 3;
                    } else {
                        info!(
                            context,
                            "Final scaled-down image size: {}B ({}px).",
                            encoded.len(),
                            img_wh
                        );
                        break;
                    }
                }
            }

            if do_scale || exif.is_some() {
                // The file format is JPEG/PNG now, we may have to change the file extension
                if !matches!(fmt, ImageFormat::Jpeg)
                    && matches!(ofmt, ImageOutputFormat::Jpeg { .. })
                {
                    name = Path::new(&name)
                        .with_extension("jpg")
                        .to_string_lossy()
                        .into_owned();
                }

                if encoded.is_empty() {
                    if mem::take(&mut add_white_bg) {
                        self::add_white_bg(&mut img);
                    }
                    encode_img(&img, ofmt, &mut encoded)?;
                }

                self.name = BlobObject::create_and_deduplicate_from_bytes(context, &encoded, &name)
                    .context("failed to write recoded blob to file")?
                    .name;
            }

            Ok(name)
        });
        match res {
            Ok(_) => res,
            Err(err) => {
                if !strict_limits && no_exif {
                    warn!(
                        context,
                        "Cannot recode image, using original data: {err:#}.",
                    );
                    Ok(original_name)
                } else {
                    Err(err)
                }
            }
        }
    }
}

fn file_hash(src: &Path) -> Result<blake3::Hash> {
    ensure!(
        !src.starts_with("$BLOBDIR/"),
        "Use `get_abs_path()` to get the absolute path of the blobfile"
    );
    let mut hasher = blake3::Hasher::new();
    let mut src_file = std::fs::File::open(src)
        .with_context(|| format!("Failed to open file {}", src.display()))?;
    hasher
        .update_reader(&mut src_file)
        .context("update_reader")?;
    let hash = hasher.finalize();
    Ok(hash)
}

/// Returns image file size and Exif.
fn image_metadata(file: &std::fs::File) -> Result<(u64, Option<exif::Exif>)> {
    let len = file.metadata()?.len();
    let mut bufreader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut bufreader).ok();
    Ok((len, exif))
}

fn exif_orientation(exif: &exif::Exif, context: &Context) -> i32 {
    if let Some(orientation) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
        // possible orientation values are described at http://sylvana.net/jpegcrop/exif_orientation.html
        // we only use rotation, in practise, flipping is not used.
        match orientation.value.get_uint(0) {
            Some(3) => return 180,
            Some(6) => return 90,
            Some(8) => return 270,
            other => warn!(context, "Exif orientation value ignored: {other:?}."),
        }
    }
    0
}

/// All files in the blobdir.
///
/// This exists so we can have a [`BlobDirIter`] which needs something to own the data of
/// it's `&Path`.  Use [`BlobDirContents::iter`] to create the iterator.
///
/// Additionally pre-allocating this means we get a length for progress report.
pub(crate) struct BlobDirContents<'a> {
    inner: Vec<PathBuf>,
    context: &'a Context,
}

impl<'a> BlobDirContents<'a> {
    pub(crate) async fn new(context: &'a Context) -> Result<BlobDirContents<'a>> {
        let readdir = fs::read_dir(context.get_blobdir()).await?;
        let inner = ReadDirStream::new(readdir)
            .filter_map(|entry| async move {
                match entry {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        error!(context, "Failed to read blob file: {err}.");
                        None
                    }
                }
            })
            .filter_map(|entry| async move {
                match entry.file_type().await.ok()?.is_file() {
                    true => Some(entry.path()),
                    false => {
                        warn!(
                            context,
                            "Export: Found blob dir entry {} that is not a file, ignoring.",
                            entry.path().display()
                        );
                        None
                    }
                }
            })
            .collect()
            .await;
        Ok(Self { inner, context })
    }

    pub(crate) fn iter(&self) -> BlobDirIter<'_> {
        BlobDirIter::new(self.context, self.inner.iter())
    }
}

/// A iterator over all the [`BlobObject`]s in the blobdir.
pub(crate) struct BlobDirIter<'a> {
    iter: std::slice::Iter<'a, PathBuf>,
    context: &'a Context,
}

impl<'a> BlobDirIter<'a> {
    fn new(context: &'a Context, iter: std::slice::Iter<'a, PathBuf>) -> BlobDirIter<'a> {
        Self { iter, context }
    }
}

impl<'a> Iterator for BlobDirIter<'a> {
    type Item = BlobObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for path in self.iter.by_ref() {
            // In theory this can error but we'd have corrupted filenames in the blobdir, so
            // silently skipping them is fine.
            match BlobObject::from_path(self.context, path) {
                Ok(blob) => return Some(blob),
                Err(err) => warn!(self.context, "{err}"),
            }
        }
        None
    }
}

impl FusedIterator for BlobDirIter<'_> {}

fn encode_img(
    img: &DynamicImage,
    fmt: ImageOutputFormat,
    encoded: &mut Vec<u8>,
) -> anyhow::Result<()> {
    encoded.clear();
    let mut buf = Cursor::new(encoded);
    match fmt {
        ImageOutputFormat::Png => img.write_to(&mut buf, ImageFormat::Png)?,
        ImageOutputFormat::Jpeg { quality } => {
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            // Convert image into RGB8 to avoid the error
            // "The encoder or decoder for Jpeg does not support the color type Rgba8"
            // (<https://github.com/image-rs/image/issues/2211>).
            img.clone().into_rgb8().write_with_encoder(encoder)?;
        }
    }
    Ok(())
}

fn encoded_img_exceeds_bytes(
    context: &Context,
    img: &DynamicImage,
    fmt: ImageOutputFormat,
    max_bytes: usize,
    encoded: &mut Vec<u8>,
) -> anyhow::Result<bool> {
    encode_img(img, fmt, encoded)?;
    if encoded.len() > max_bytes {
        info!(
            context,
            "Image size {}B ({}x{}px) exceeds {}B, need to scale down.",
            encoded.len(),
            img.width(),
            img.height(),
            max_bytes,
        );
        return Ok(true);
    }
    Ok(false)
}

/// Removes transparency from an image using a white background.
fn add_white_bg(img: &mut DynamicImage) {
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut p = Rgba([255u8, 255, 255, 255]);
            p.blend(&img.get_pixel(x, y));
            img.put_pixel(x, y, p);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::message::{Message, Viewtype};
    use crate::sql;
    use crate::test_utils::{self, TestContext};
    use crate::tools::SystemTime;

    fn check_image_size(path: impl AsRef<Path>, width: u32, height: u32) -> image::DynamicImage {
        tokio::task::block_in_place(move || {
            let img = ImageReader::open(path)
                .expect("failed to open image")
                .with_guessed_format()
                .expect("failed to guess format")
                .decode()
                .expect("failed to decode image");
            assert_eq!(img.width(), width, "invalid width");
            assert_eq!(img.height(), height, "invalid height");
            img
        })
    }

    const FILE_BYTES: &[u8] = b"hello";
    const FILE_DEDUPLICATED: &str = "ea8f163db38682925e4491c5e58d4bb.txt";

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create() {
        let t = TestContext::new().await;
        let blob =
            BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.txt").unwrap();
        let fname = t.get_blobdir().join(FILE_DEDUPLICATED);
        let data = fs::read(fname).await.unwrap();
        assert_eq!(data, FILE_BYTES);
        assert_eq!(blob.as_name(), format!("$BLOBDIR/{FILE_DEDUPLICATED}"));
        assert_eq!(blob.to_abs_path(), t.get_blobdir().join(FILE_DEDUPLICATED));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lowercase_ext() {
        let t = TestContext::new().await;
        let blob =
            BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.TXT").unwrap();
        assert!(
            blob.as_name().ends_with(".txt"),
            "Blob {blob:?} should end with .txt"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_as_file_name() {
        let t = TestContext::new().await;
        let blob =
            BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.txt").unwrap();
        assert_eq!(blob.as_file_name(), FILE_DEDUPLICATED);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_as_rel_path() {
        let t = TestContext::new().await;
        let blob =
            BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.txt").unwrap();
        assert_eq!(blob.as_rel_path(), Path::new(FILE_DEDUPLICATED));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_suffix() {
        let t = TestContext::new().await;
        let blob =
            BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.txt").unwrap();
        assert_eq!(blob.suffix(), Some("txt"));
        let blob = BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "bar").unwrap();
        assert_eq!(blob.suffix(), None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_dup() {
        let t = TestContext::new().await;
        BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.txt").unwrap();
        let foo_path = t.get_blobdir().join(FILE_DEDUPLICATED);
        assert!(foo_path.exists());
        BlobObject::create_and_deduplicate_from_bytes(&t, b"world", "foo.txt").unwrap();
        let mut dir = fs::read_dir(t.get_blobdir()).await.unwrap();
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let fname = dirent.file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).await.unwrap(), FILE_BYTES);
            } else {
                let name = fname.to_str().unwrap();
                assert!(name.ends_with(".txt"));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_double_ext() {
        let t = TestContext::new().await;
        BlobObject::create_and_deduplicate_from_bytes(&t, FILE_BYTES, "foo.tar.gz").unwrap();
        let foo_path = t.get_blobdir().join(FILE_DEDUPLICATED).with_extension("gz");
        assert!(foo_path.exists());
        BlobObject::create_and_deduplicate_from_bytes(&t, b"world", "foo.tar.gz").unwrap();
        let mut dir = fs::read_dir(t.get_blobdir()).await.unwrap();
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let fname = dirent.file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).await.unwrap(), FILE_BYTES);
            } else {
                let name = fname.to_str().unwrap();
                println!("{name}");
                assert_eq!(name.starts_with("foo"), false);
                assert_eq!(name.ends_with(".tar.gz"), false);
                assert!(name.ends_with(".gz"));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_long_names() {
        let t = TestContext::new().await;
        let s = format!("file.{}", "a".repeat(100));
        let blob = BlobObject::create_and_deduplicate_from_bytes(&t, b"data", &s).unwrap();
        let blobname = blob.as_name().split('/').last().unwrap();
        assert!(blobname.len() < 70);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_and_copy() {
        let t = TestContext::new().await;
        let src = t.dir.path().join("src");
        fs::write(&src, b"boo").await.unwrap();
        let blob = BlobObject::create_and_copy(&t, src.as_ref()).await.unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/src");
        let data = fs::read(blob.to_abs_path()).await.unwrap();
        assert_eq!(data, b"boo");

        let whoops = t.dir.path().join("whoops");
        assert!(BlobObject::create_and_copy(&t, whoops.as_ref())
            .await
            .is_err());
        let whoops = t.get_blobdir().join("whoops");
        assert!(!whoops.exists());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_from_path() {
        let t = TestContext::new().await;

        let src_ext = t.dir.path().join("external");
        fs::write(&src_ext, b"boo").await.unwrap();
        let blob = BlobObject::new_from_path(&t, src_ext.as_ref())
            .await
            .unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/external");
        let data = fs::read(blob.to_abs_path()).await.unwrap();
        assert_eq!(data, b"boo");

        let src_int = t.get_blobdir().join("internal");
        fs::write(&src_int, b"boo").await.unwrap();
        let blob = BlobObject::new_from_path(&t, &src_int).await.unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/internal");
        let data = fs::read(blob.to_abs_path()).await.unwrap();
        assert_eq!(data, b"boo");
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_from_name_long() {
        let t = TestContext::new().await;
        let src_ext = t.dir.path().join("autocrypt-setup-message-4137848473.html");
        fs::write(&src_ext, b"boo").await.unwrap();
        let blob = BlobObject::new_from_path(&t, src_ext.as_ref())
            .await
            .unwrap();
        assert_eq!(
            blob.as_name(),
            "$BLOBDIR/autocrypt-setup-message-4137848473.html"
        );
    }

    #[test]
    fn test_is_blob_name() {
        assert!(BlobObject::is_acceptible_blob_name("foo"));
        assert!(BlobObject::is_acceptible_blob_name("foo.txt"));
        assert!(BlobObject::is_acceptible_blob_name("f".repeat(128)));
        assert!(!BlobObject::is_acceptible_blob_name("foo/bar"));
        assert!(!BlobObject::is_acceptible_blob_name("foo\\bar"));
        assert!(!BlobObject::is_acceptible_blob_name("foo\x00bar"));
    }

    #[test]
    fn test_sanitise_name() {
        let (stem, ext) =
            BlobObject::sanitise_name("Я ЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯ.txt");
        assert_eq!(ext, ".txt");
        assert!(!stem.is_empty());

        // the extensions are kept together as between stem and extension a number may be added -
        // and `foo.tar.gz` should become `foo-1234.tar.gz` and not `foo.tar-1234.gz`
        let (stem, ext) = BlobObject::sanitise_name("wot.tar.gz");
        assert_eq!(stem, "wot");
        assert_eq!(ext, ".tar.gz");

        let (stem, ext) = BlobObject::sanitise_name(".foo.bar");
        assert_eq!(stem, "");
        assert_eq!(ext, ".foo.bar");

        let (stem, ext) = BlobObject::sanitise_name("foo?.bar");
        assert!(stem.contains("foo"));
        assert!(!stem.contains('?'));
        assert_eq!(ext, ".bar");

        let (stem, ext) = BlobObject::sanitise_name("no-extension");
        assert_eq!(stem, "no-extension");
        assert_eq!(ext, "");

        let (stem, ext) = BlobObject::sanitise_name("path/ignored\\this: is* forbidden?.c");
        assert_eq!(ext, ".c");
        assert!(!stem.contains("path"));
        assert!(!stem.contains("ignored"));
        assert!(stem.contains("this"));
        assert!(stem.contains("forbidden"));
        assert!(!stem.contains('/'));
        assert!(!stem.contains('\\'));
        assert!(!stem.contains(':'));
        assert!(!stem.contains('*'));
        assert!(!stem.contains('?'));

        let (stem, ext) = BlobObject::sanitise_name(
            "file.with_lots_of_characters_behind_point_and_double_ending.tar.gz",
        );
        assert_eq!(
            stem,
            "file.with_lots_of_characters_behind_point_and_double_ending"
        );
        assert_eq!(ext, ".tar.gz");

        let (stem, ext) = BlobObject::sanitise_name("a. tar.tar.gz");
        assert_eq!(stem, "a. tar");
        assert_eq!(ext, ".tar.gz");

        let (stem, ext) = BlobObject::sanitise_name("Guia_uso_GNB (v0.8).pdf");
        assert_eq!(stem, "Guia_uso_GNB (v0.8)");
        assert_eq!(ext, ".pdf");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_add_white_bg() {
        let t = TestContext::new().await;
        let bytes0 = include_bytes!("../test-data/image/logo.png").as_slice();
        let bytes1 = include_bytes!("../test-data/image/avatar900x900.png").as_slice();
        for (bytes, color) in [
            (bytes0, [255u8, 255, 255, 255]),
            (bytes1, [253u8, 198, 0, 255]),
        ] {
            let avatar_src = t.dir.path().join("avatar.png");
            fs::write(&avatar_src, bytes).await.unwrap();

            let mut blob = BlobObject::new_from_path(&t, &avatar_src).await.unwrap();
            let img_wh = 128;
            let maybe_sticker = &mut false;
            let strict_limits = true;
            blob.recode_to_size(&t, None, maybe_sticker, img_wh, 20_000, strict_limits)
                .unwrap();
            tokio::task::block_in_place(move || {
                let img = ImageReader::open(blob.to_abs_path())
                    .unwrap()
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();
                assert!(img.width() == img_wh);
                assert!(img.height() == img_wh);
                assert_eq!(img.get_pixel(0, 0), Rgba(color));
            });
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_outside_blobdir() {
        async fn file_size(path_buf: &Path) -> u64 {
            fs::metadata(path_buf).await.unwrap().len()
        }

        let t = TestContext::new().await;
        let avatar_src = t.dir.path().join("avatar.jpg");
        let avatar_bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        fs::write(&avatar_src, avatar_bytes).await.unwrap();
        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        let avatar_blob = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        let avatar_path = Path::new(&avatar_blob);
        assert!(
            avatar_blob.ends_with("d98cd30ed8f2129bf3968420208849d.jpg"),
            "The avatar filename should be its hash, put instead it's {avatar_blob}"
        );
        let scaled_avatar_size = file_size(avatar_path).await;
        assert!(scaled_avatar_size < avatar_bytes.len() as u64);

        check_image_size(avatar_src, 1000, 1000);
        check_image_size(
            &avatar_blob,
            constants::BALANCED_AVATAR_SIZE,
            constants::BALANCED_AVATAR_SIZE,
        );

        let mut blob = BlobObject::new_from_path(&t, avatar_path).await.unwrap();
        let maybe_sticker = &mut false;
        let strict_limits = true;
        blob.recode_to_size(&t, None, maybe_sticker, 1000, 3000, strict_limits)
            .unwrap();
        let new_file_size = file_size(&blob.to_abs_path()).await;
        assert!(new_file_size <= 3000);
        assert!(new_file_size > 2000);
        // The new file should be smaller:
        assert!(new_file_size < scaled_avatar_size);
        // And the original file should not be touched:
        assert_eq!(file_size(avatar_path).await, scaled_avatar_size);
        tokio::task::block_in_place(move || {
            let img = ImageReader::open(blob.to_abs_path())
                .unwrap()
                .with_guessed_format()
                .unwrap()
                .decode()
                .unwrap();
            assert!(img.width() > 130);
            assert_eq!(img.width(), img.height());
        });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_in_blobdir() {
        let t = TestContext::new().await;
        let avatar_src = t.get_blobdir().join("avatar.png");
        fs::write(&avatar_src, test_utils::AVATAR_900x900_BYTES)
            .await
            .unwrap();

        check_image_size(&avatar_src, 900, 900);

        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        let avatar_cfg = t.get_config(Config::Selfavatar).await.unwrap().unwrap();
        assert!(
            avatar_cfg.ends_with("9e7f409ac5c92b942cc4f31cee2770a.png"),
            "Avatar file name {avatar_cfg} should end with its hash"
        );

        check_image_size(
            avatar_cfg,
            constants::BALANCED_AVATAR_SIZE,
            constants::BALANCED_AVATAR_SIZE,
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_copy_without_recode() {
        let t = TestContext::new().await;
        let avatar_src = t.dir.path().join("avatar.png");
        let avatar_bytes = include_bytes!("../test-data/image/avatar64x64.png");
        fs::write(&avatar_src, avatar_bytes).await.unwrap();
        let avatar_blob = t.get_blobdir().join("e9b6c7a78aa2e4f415644f55a553e73.png");
        assert!(!avatar_blob.exists());
        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        assert!(avatar_blob.exists());
        assert_eq!(
            fs::metadata(&avatar_blob).await.unwrap().len(),
            avatar_bytes.len() as u64
        );
        let avatar_cfg = t.get_config(Config::Selfavatar).await.unwrap();
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_1() {
        let bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "0",
            bytes,
            extension: "jpg",
            has_exif: true,
            original_width: 1000,
            original_height: 1000,
            compressed_width: 1000,
            compressed_height: 1000,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "1",
            bytes,
            extension: "jpg",
            has_exif: true,
            original_width: 1000,
            original_height: 1000,
            compressed_width: 1000,
            compressed_height: 1000,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_2() {
        // The "-rotated" files are rotated by 270 degrees using the Exif metadata
        let bytes = include_bytes!("../test-data/image/rectangle2000x1800-rotated.jpg");
        let img_rotated = SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "0",
            bytes,
            extension: "jpg",
            has_exif: true,
            original_width: 2000,
            original_height: 1800,
            orientation: 270,
            compressed_width: 1800,
            compressed_height: 2000,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
        assert_correct_rotation(&img_rotated);

        let mut buf = Cursor::new(vec![]);
        img_rotated.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
        let bytes = buf.into_inner();

        let img_rotated = SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "1",
            bytes: &bytes,
            extension: "jpg",
            original_width: 1800,
            original_height: 2000,
            compressed_width: 1800,
            compressed_height: 2000,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
        assert_correct_rotation(&img_rotated);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_balanced_png() {
        let bytes = include_bytes!("../test-data/image/screenshot.png");

        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "0",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: 1920,
            compressed_height: 1080,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();

        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "1",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: constants::WORSE_IMAGE_SIZE,
            compressed_height: constants::WORSE_IMAGE_SIZE * 1080 / 1920,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();

        SendImageCheckMediaquality {
            viewtype: Viewtype::File,
            media_quality_config: "1",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: 1920,
            compressed_height: 1080,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();

        SendImageCheckMediaquality {
            viewtype: Viewtype::File,
            media_quality_config: "1",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: 1920,
            compressed_height: 1080,
            set_draft: true,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();

        // This will be sent as Image, see [`BlobObject::maybe_sticker`] for explanation.
        SendImageCheckMediaquality {
            viewtype: Viewtype::Sticker,
            media_quality_config: "0",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: 1920,
            compressed_height: 1080,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
    }

    /// Tests that RGBA PNG can be recoded into JPEG
    /// by dropping alpha channel.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_rgba_png_to_jpeg() {
        let bytes = include_bytes!("../test-data/image/screenshot-rgba.png");

        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "1",
            bytes,
            extension: "png",
            original_width: 1920,
            original_height: 1080,
            compressed_width: constants::WORSE_IMAGE_SIZE,
            compressed_height: constants::WORSE_IMAGE_SIZE * 1080 / 1920,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_huge_jpg() {
        let bytes = include_bytes!("../test-data/image/screenshot.jpg");
        SendImageCheckMediaquality {
            viewtype: Viewtype::Image,
            media_quality_config: "0",
            bytes,
            extension: "jpg",
            has_exif: true,
            original_width: 1920,
            original_height: 1080,
            compressed_width: constants::BALANCED_IMAGE_SIZE,
            compressed_height: constants::BALANCED_IMAGE_SIZE * 1080 / 1920,
            ..Default::default()
        }
        .test()
        .await
        .unwrap();
    }

    fn assert_correct_rotation(img: &DynamicImage) {
        // The test images are black in the bottom left corner after correctly applying
        // the EXIF orientation

        let [luma] = img.get_pixel(10, 10).to_luma().0;
        assert_eq!(luma, 255);
        let [luma] = img.get_pixel(img.width() - 10, 10).to_luma().0;
        assert_eq!(luma, 255);
        let [luma] = img
            .get_pixel(img.width() - 10, img.height() - 10)
            .to_luma()
            .0;
        assert_eq!(luma, 255);
        let [luma] = img.get_pixel(10, img.height() - 10).to_luma().0;
        assert_eq!(luma, 0);
    }

    #[derive(Default)]
    struct SendImageCheckMediaquality<'a> {
        pub(crate) viewtype: Viewtype,
        pub(crate) media_quality_config: &'a str,
        pub(crate) bytes: &'a [u8],
        pub(crate) extension: &'a str,
        pub(crate) has_exif: bool,
        pub(crate) original_width: u32,
        pub(crate) original_height: u32,
        pub(crate) orientation: i32,
        pub(crate) compressed_width: u32,
        pub(crate) compressed_height: u32,
        pub(crate) set_draft: bool,
    }

    impl SendImageCheckMediaquality<'_> {
        pub(crate) async fn test(self) -> anyhow::Result<DynamicImage> {
            let viewtype = self.viewtype;
            let media_quality_config = self.media_quality_config;
            let bytes = self.bytes;
            let extension = self.extension;
            let has_exif = self.has_exif;
            let original_width = self.original_width;
            let original_height = self.original_height;
            let orientation = self.orientation;
            let compressed_width = self.compressed_width;
            let compressed_height = self.compressed_height;
            let set_draft = self.set_draft;

            let alice = TestContext::new_alice().await;
            let bob = TestContext::new_bob().await;
            alice
                .set_config(Config::MediaQuality, Some(media_quality_config))
                .await?;
            let file = alice.get_blobdir().join("file").with_extension(extension);
            let file_name = format!("file.{extension}");

            fs::write(&file, &bytes)
                .await
                .context("failed to write file")?;
            check_image_size(&file, original_width, original_height);

            let (_, exif) = image_metadata(&std::fs::File::open(&file)?)?;
            if has_exif {
                let exif = exif.unwrap();
                assert_eq!(exif_orientation(&exif, &alice), orientation);
            } else {
                assert!(exif.is_none());
            }

            let mut msg = Message::new(viewtype);
            msg.set_file_and_deduplicate(&alice, &file, Some(&file_name), None)?;
            let chat = alice.create_chat(&bob).await;
            if set_draft {
                chat.id.set_draft(&alice, Some(&mut msg)).await.unwrap();
                msg = chat.id.get_draft(&alice).await.unwrap().unwrap();
                assert_eq!(msg.get_viewtype(), Viewtype::File);
            }
            let sent = alice.send_msg(chat.id, &mut msg).await;
            let alice_msg = alice.get_last_msg().await;
            assert_eq!(alice_msg.get_width() as u32, compressed_width);
            assert_eq!(alice_msg.get_height() as u32, compressed_height);
            let file_saved = alice
                .get_blobdir()
                .join("saved-".to_string() + &alice_msg.get_filename().unwrap());
            alice_msg.save_file(&alice, &file_saved).await?;
            check_image_size(file_saved, compressed_width, compressed_height);

            let bob_msg = bob.recv_msg(&sent).await;
            assert_eq!(bob_msg.get_viewtype(), Viewtype::Image);
            assert_eq!(bob_msg.get_width() as u32, compressed_width);
            assert_eq!(bob_msg.get_height() as u32, compressed_height);
            let file_saved = bob
                .get_blobdir()
                .join("saved-".to_string() + &bob_msg.get_filename().unwrap());
            bob_msg.save_file(&bob, &file_saved).await?;
            if viewtype == Viewtype::File {
                assert_eq!(file_saved.extension().unwrap(), extension);
                let bytes1 = fs::read(&file_saved).await?;
                assert_eq!(&bytes1, bytes);
            }

            let (_, exif) = image_metadata(&std::fs::File::open(&file_saved)?)?;
            assert!(exif.is_none());

            let img = check_image_size(file_saved, compressed_width, compressed_height);
            Ok(img)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_big_gif_as_image() -> Result<()> {
        let bytes = include_bytes!("../test-data/image/screenshot.gif");
        let (width, height) = (1920u32, 1080u32);
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        alice
            .set_config(
                Config::MediaQuality,
                Some(&(MediaQuality::Worse as i32).to_string()),
            )
            .await?;
        let file = alice.get_blobdir().join("file").with_extension("gif");
        fs::write(&file, &bytes)
            .await
            .context("failed to write file")?;
        let mut msg = Message::new(Viewtype::Image);
        msg.set_file_and_deduplicate(&alice, &file, Some("file.gif"), None)?;
        let chat = alice.create_chat(&bob).await;
        let sent = alice.send_msg(chat.id, &mut msg).await;
        let bob_msg = bob.recv_msg(&sent).await;
        // DC must detect the image as GIF and send it w/o reencoding.
        assert_eq!(bob_msg.get_viewtype(), Viewtype::Gif);
        assert_eq!(bob_msg.get_width() as u32, width);
        assert_eq!(bob_msg.get_height() as u32, height);
        let file_saved = bob
            .get_blobdir()
            .join("saved-".to_string() + &bob_msg.get_filename().unwrap());
        bob_msg.save_file(&bob, &file_saved).await?;
        let (file_size, _) = image_metadata(&std::fs::File::open(&file_saved)?)?;
        assert_eq!(file_size, bytes.len() as u64);
        check_image_size(file_saved, width, height);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_gif_as_sticker() -> Result<()> {
        let bytes = include_bytes!("../test-data/image/image100x50.gif");
        let alice = &TestContext::new_alice().await;
        let file = alice.get_blobdir().join("file").with_extension("gif");
        fs::write(&file, &bytes)
            .await
            .context("failed to write file")?;
        let mut msg = Message::new(Viewtype::Sticker);
        msg.set_file_and_deduplicate(alice, &file, None, None)?;
        let chat = alice.get_self_chat().await;
        let sent = alice.send_msg(chat.id, &mut msg).await;
        let msg = Message::load_from_db(alice, sent.sender_msg_id).await?;
        // Message::force_sticker() wasn't used, still Viewtype::Sticker is preserved because of the
        // extension.
        assert_eq!(msg.get_viewtype(), Viewtype::Sticker);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_and_deduplicate() -> Result<()> {
        let t = TestContext::new().await;

        let path = t.get_blobdir().join("anyfile.dat");
        fs::write(&path, b"bla").await?;
        let blob = BlobObject::create_and_deduplicate(&t, &path, &path)?;
        assert_eq!(blob.name, "$BLOBDIR/ce940175885d7b78f7b7e9f1396611f.dat");
        assert_eq!(path.exists(), false);

        assert_eq!(fs::read(&blob.to_abs_path()).await?, b"bla");

        fs::write(&path, b"bla").await?;
        let blob2 = BlobObject::create_and_deduplicate(&t, &path, &path)?;
        assert_eq!(blob2.name, blob.name);

        let path_outside_blobdir = t.dir.path().join("anyfile.dat");
        fs::write(&path_outside_blobdir, b"bla").await?;
        let blob3 =
            BlobObject::create_and_deduplicate(&t, &path_outside_blobdir, &path_outside_blobdir)?;
        assert!(path_outside_blobdir.exists());
        assert_eq!(blob3.name, blob.name);

        fs::write(&path, b"blabla").await?;
        let blob4 = BlobObject::create_and_deduplicate(&t, &path, &path)?;
        assert_ne!(blob4.name, blob.name);

        fs::remove_dir_all(t.get_blobdir()).await?;
        let blob5 =
            BlobObject::create_and_deduplicate(&t, &path_outside_blobdir, &path_outside_blobdir)?;
        assert_eq!(blob5.name, blob.name);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_and_deduplicate_from_bytes() -> Result<()> {
        let t = TestContext::new().await;

        fs::remove_dir(t.get_blobdir()).await?;
        let blob = BlobObject::create_and_deduplicate_from_bytes(&t, b"bla", "file")?;
        assert_eq!(blob.name, "$BLOBDIR/ce940175885d7b78f7b7e9f1396611f");

        assert_eq!(fs::read(&blob.to_abs_path()).await?, b"bla");
        let modified1 = blob.to_abs_path().metadata()?.modified()?;

        // Test that the modification time of the file is updated when a new file is created
        // so that it's not deleted during housekeeping.
        // We can't use SystemTime::shift() here because file creation uses the actual OS time,
        // which we can't mock from our code.
        tokio::time::sleep(Duration::from_millis(1100)).await;

        let blob2 = BlobObject::create_and_deduplicate_from_bytes(&t, b"bla", "file")?;
        assert_eq!(blob2.name, blob.name);

        let modified2 = blob.to_abs_path().metadata()?.modified()?;
        assert_ne!(modified1, modified2);
        sql::housekeeping(&t).await?;
        assert!(blob2.to_abs_path().exists());

        // If we do shift the time by more than 1h, the blob file will be deleted during housekeeping:
        SystemTime::shift(Duration::from_secs(65 * 60));
        sql::housekeeping(&t).await?;
        assert_eq!(blob2.to_abs_path().exists(), false);

        let blob3 = BlobObject::create_and_deduplicate_from_bytes(&t, b"blabla", "file")?;
        assert_ne!(blob3.name, blob.name);

        {
            // If something goes wrong and the blob file is overwritten,
            // the correct content should be restored:
            fs::write(blob3.to_abs_path(), b"bloblo").await?;

            let blob4 = BlobObject::create_and_deduplicate_from_bytes(&t, b"blabla", "file")?;
            let blob4_content = fs::read(blob4.to_abs_path()).await?;
            assert_eq!(blob4_content, b"blabla");
        }

        Ok(())
    }
}
