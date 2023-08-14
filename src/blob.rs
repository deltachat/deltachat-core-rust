//! # Blob directory management.

use core::cmp::max;
use std::ffi::OsStr;
use std::fmt;
use std::io::Cursor;
use std::iter::FusedIterator;
use std::path::{Path, PathBuf};

use anyhow::{format_err, Context as _, Result};
use futures::StreamExt;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageOutputFormat};
use num_traits::FromPrimitive;
use tokio::io::AsyncWriteExt;
use tokio::{fs, io};
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
    name: String,
}

impl<'a> BlobObject<'a> {
    /// Creates a new blob object with a unique name.
    ///
    /// Creates a new file in the blob directory.  The name will be
    /// derived from the platform-agnostic basename of the suggested
    /// name, followed by a random number and followed by a possible
    /// extension.  The `data` will be written into the file without
    /// race-conditions.
    pub async fn create(
        context: &'a Context,
        suggested_name: &str,
        data: &[u8],
    ) -> Result<BlobObject<'a>> {
        let blobdir = context.get_blobdir();
        let (stem, ext) = BlobObject::sanitise_name(suggested_name);
        let (name, mut file) = BlobObject::create_new_file(context, blobdir, &stem, &ext).await?;
        file.write_all(data).await.context("file write failure")?;

        // workaround a bug in async-std
        // (the executor does not handle blocking operation in Drop correctly,
        // see <https://github.com/async-rs/async-std/issues/900>)
        let _ = file.flush().await;

        let blob = BlobObject {
            blobdir,
            name: format!("$BLOBDIR/{name}"),
        };
        context.emit_event(EventType::NewBlobFile(blob.as_name().to_string()));
        Ok(blob)
    }

    // Creates a new file, returning a tuple of the name and the handle.
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
    /// This creates a new blob as described in [BlobObject::create]
    /// but also copies an existing file into it.  This is done in a
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

        // workaround, see create() for details
        let _ = dst_file.flush().await;

        let blob = BlobObject {
            blobdir: context.get_blobdir(),
            name: format!("$BLOBDIR/{name}"),
        };
        context.emit_event(EventType::NewBlobFile(blob.as_name().to_string()));
        Ok(blob)
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
        let mut name = name.to_string();
        for part in name.rsplit('/') {
            if !part.is_empty() {
                name = part.to_string();
                break;
            }
        }
        for part in name.rsplit('\\') {
            if !part.is_empty() {
                name = part.to_string();
                break;
            }
        }
        let opts = sanitize_filename::Options {
            truncate: true,
            windows: true,
            replacement: "",
        };

        let clean = sanitize_filename::sanitize_with_options(name, opts);
        // Let's take the tricky filename
        // "file.with_lots_of_characters_behind_point_and_double_ending.tar.gz" as an example.
        // Split it into "file" and "with_lots_of_characters_behind_point_and_double_ending.tar.gz":
        let mut iter = clean.splitn(2, '.');

        let stem: String = iter.next().unwrap_or_default().chars().take(64).collect();
        // stem == "file"

        let ext_chars = iter.next().unwrap_or_default().chars();
        let ext: String = ext_chars
            .rev()
            .take(32)
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .collect();
        // ext == "d_point_and_double_ending.tar.gz"

        if ext.is_empty() {
            (stem, "".to_string())
        } else {
            (stem, format!(".{ext}").to_lowercase())
            // Return ("file", ".d_point_and_double_ending.tar.gz")
            // which is not perfect but acceptable.
        }
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

    pub async fn recode_to_avatar_size(&mut self, context: &Context) -> Result<()> {
        let blob_abs = self.to_abs_path();

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
        if let Some(new_name) = self.recode_to_size(
            context,
            blob_abs,
            maybe_sticker,
            img_wh,
            20_000,
            strict_limits,
        )? {
            self.name = new_name;
        }
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
        maybe_sticker: &mut bool,
    ) -> Result<()> {
        let blob_abs = self.to_abs_path();
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
        if let Some(new_name) = self.recode_to_size(
            context,
            blob_abs,
            maybe_sticker,
            img_wh,
            max_bytes,
            strict_limits,
        )? {
            self.name = new_name;
        }
        Ok(())
    }

    /// If `!strict_limits`, then if `max_bytes` is exceeded, reduce the image to `img_wh` and just
    /// proceed with the result.
    fn recode_to_size(
        &mut self,
        context: &Context,
        mut blob_abs: PathBuf,
        maybe_sticker: &mut bool,
        mut img_wh: u32,
        max_bytes: usize,
        strict_limits: bool,
    ) -> Result<Option<String>> {
        tokio::task::block_in_place(move || {
            let mut img = image::open(&blob_abs).context("image decode failure")?;
            let (nr_bytes, exif) = self.metadata()?;
            let orientation = exif.as_ref().map(|exif| exif_orientation(exif, context));
            let mut encoded = Vec::new();
            let mut changed_name = None;

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
                return Ok(None);
            }

            img = match orientation {
                Some(90) => img.rotate90(),
                Some(180) => img.rotate180(),
                Some(270) => img.rotate270(),
                _ => img,
            };

            let exceeds_wh = img.width() > img_wh || img.height() > img_wh;
            let exceeds_max_bytes = nr_bytes > max_bytes as u64;

            let fmt = ImageFormat::from_path(&blob_abs);
            let ofmt = match fmt {
                Ok(ImageFormat::Png) if !exceeds_max_bytes => ImageOutputFormat::Png,
                _ => {
                    let jpeg_quality = 75;
                    ImageOutputFormat::Jpeg(jpeg_quality)
                }
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
                        || exif.is_some()
                            && encoded_img_exceeds_bytes(
                                context,
                                &img,
                                ofmt.clone(),
                                max_bytes,
                                &mut encoded,
                            )?);

            if do_scale {
                if !exceeds_wh {
                    img_wh = max(img.width(), img.height());
                    // PNGs and WebPs may be huge because of animation, which is lost by the `image`
                    // crate when recoding, so don't scale them down.
                    if matches!(fmt, Ok(ImageFormat::Jpeg)) || !encoded.is_empty() {
                        img_wh = img_wh * 2 / 3;
                    }
                }

                loop {
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
                if !matches!(fmt, Ok(ImageFormat::Jpeg))
                    && matches!(ofmt, ImageOutputFormat::Jpeg(_))
                {
                    blob_abs = blob_abs.with_extension("jpg");
                    let file_name = blob_abs.file_name().context("No image file name (???)")?;
                    let file_name = file_name.to_str().context("Filename is no UTF-8 (???)")?;
                    changed_name = Some(format!("$BLOBDIR/{file_name}"));
                }

                if encoded.is_empty() {
                    encode_img(&img, ofmt, &mut encoded)?;
                }

                std::fs::write(&blob_abs, &encoded)
                    .context("failed to write recoded blob to file")?;
            }

            Ok(changed_name)
        })
    }

    /// Returns image file size and Exif.
    pub fn metadata(&self) -> Result<(u64, Option<exif::Exif>)> {
        let file = std::fs::File::open(self.to_abs_path())?;
        let len = file.metadata()?.len();
        let mut bufreader = std::io::BufReader::new(&file);
        let exif = exif::Reader::new().read_from_container(&mut bufreader).ok();
        Ok((len, exif))
    }
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

impl<'a> fmt::Display for BlobObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$BLOBDIR/{}", self.name)
    }
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

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
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
    img.write_to(&mut buf, fmt)?;
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use fs::File;
    use image::{GenericImageView, Pixel};

    use super::*;
    use crate::chat::{self, create_group_chat, ProtectionStatus};
    use crate::message::{Message, Viewtype};
    use crate::test_utils::{self, TestContext};

    fn check_image_size(path: impl AsRef<Path>, width: u32, height: u32) -> image::DynamicImage {
        tokio::task::block_in_place(move || {
            let img = image::open(path).expect("failed to open image");
            assert_eq!(img.width(), width, "invalid width");
            assert_eq!(img.height(), height, "invalid height");
            img
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create() {
        let t = TestContext::new().await;
        let blob = BlobObject::create(&t, "foo", b"hello").await.unwrap();
        let fname = t.get_blobdir().join("foo");
        let data = fs::read(fname).await.unwrap();
        assert_eq!(data, b"hello");
        assert_eq!(blob.as_name(), "$BLOBDIR/foo");
        assert_eq!(blob.to_abs_path(), t.get_blobdir().join("foo"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lowercase_ext() {
        let t = TestContext::new().await;
        let blob = BlobObject::create(&t, "foo.TXT", b"hello").await.unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/foo.txt");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_as_file_name() {
        let t = TestContext::new().await;
        let blob = BlobObject::create(&t, "foo.txt", b"hello").await.unwrap();
        assert_eq!(blob.as_file_name(), "foo.txt");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_as_rel_path() {
        let t = TestContext::new().await;
        let blob = BlobObject::create(&t, "foo.txt", b"hello").await.unwrap();
        assert_eq!(blob.as_rel_path(), Path::new("foo.txt"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_suffix() {
        let t = TestContext::new().await;
        let blob = BlobObject::create(&t, "foo.txt", b"hello").await.unwrap();
        assert_eq!(blob.suffix(), Some("txt"));
        let blob = BlobObject::create(&t, "bar", b"world").await.unwrap();
        assert_eq!(blob.suffix(), None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_dup() {
        let t = TestContext::new().await;
        BlobObject::create(&t, "foo.txt", b"hello").await.unwrap();
        let foo_path = t.get_blobdir().join("foo.txt");
        assert!(foo_path.exists());
        BlobObject::create(&t, "foo.txt", b"world").await.unwrap();
        let mut dir = fs::read_dir(t.get_blobdir()).await.unwrap();
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let fname = dirent.file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).await.unwrap(), b"hello");
            } else {
                let name = fname.to_str().unwrap();
                assert!(name.starts_with("foo"));
                assert!(name.ends_with(".txt"));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_double_ext_preserved() {
        let t = TestContext::new().await;
        BlobObject::create(&t, "foo.tar.gz", b"hello")
            .await
            .unwrap();
        let foo_path = t.get_blobdir().join("foo.tar.gz");
        assert!(foo_path.exists());
        BlobObject::create(&t, "foo.tar.gz", b"world")
            .await
            .unwrap();
        let mut dir = fs::read_dir(t.get_blobdir()).await.unwrap();
        while let Ok(Some(dirent)) = dir.next_entry().await {
            let fname = dirent.file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).await.unwrap(), b"hello");
            } else {
                let name = fname.to_str().unwrap();
                println!("{name}");
                assert!(name.starts_with("foo"));
                assert!(name.ends_with(".tar.gz"));
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_long_names() {
        let t = TestContext::new().await;
        let s = "1".repeat(150);
        let blob = BlobObject::create(&t, &s, b"data").await.unwrap();
        let blobname = blob.as_name().split('/').last().unwrap();
        assert!(blobname.len() < 128);
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
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_selfavatar_outside_blobdir() {
        let t = TestContext::new().await;
        let avatar_src = t.dir.path().join("avatar.jpg");
        let avatar_bytes = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        fs::write(&avatar_src, avatar_bytes).await.unwrap();
        let avatar_blob = t.get_blobdir().join("avatar.jpg");
        assert!(!avatar_blob.exists());
        t.set_config(Config::Selfavatar, Some(avatar_src.to_str().unwrap()))
            .await
            .unwrap();
        assert!(avatar_blob.exists());
        assert!(fs::metadata(&avatar_blob).await.unwrap().len() < avatar_bytes.len() as u64);
        let avatar_cfg = t.get_config(Config::Selfavatar).await.unwrap();
        assert_eq!(avatar_cfg, avatar_blob.to_str().map(|s| s.to_string()));

        check_image_size(avatar_src, 1000, 1000);
        check_image_size(
            &avatar_blob,
            constants::BALANCED_AVATAR_SIZE,
            constants::BALANCED_AVATAR_SIZE,
        );

        async fn file_size(path_buf: &Path) -> u64 {
            let file = File::open(path_buf).await.unwrap();
            file.metadata().await.unwrap().len()
        }

        let mut blob = BlobObject::new_from_path(&t, &avatar_blob).await.unwrap();
        let maybe_sticker = &mut false;
        let strict_limits = true;
        blob.recode_to_size(
            &t,
            blob.to_abs_path(),
            maybe_sticker,
            1000,
            3000,
            strict_limits,
        )
        .unwrap();
        assert!(file_size(&avatar_blob).await <= 3000);
        assert!(file_size(&avatar_blob).await > 2000);
        tokio::task::block_in_place(move || {
            let img = image::open(avatar_blob).unwrap();
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
        assert_eq!(
            avatar_cfg,
            avatar_src.with_extension("png").to_str().unwrap()
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
        let avatar_blob = t.get_blobdir().join("avatar.png");
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
        send_image_check_mediaquality(
            Viewtype::Image,
            Some("0"),
            bytes,
            "jpg",
            true, // has Exif
            1000,
            1000,
            0,
            1000,
            1000,
        )
        .await
        .unwrap();
        send_image_check_mediaquality(
            Viewtype::Image,
            Some("1"),
            bytes,
            "jpg",
            true, // has Exif
            1000,
            1000,
            0,
            1000,
            1000,
        )
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_2() {
        // The "-rotated" files are rotated by 270 degrees using the Exif metadata
        let bytes = include_bytes!("../test-data/image/rectangle2000x1800-rotated.jpg");
        let img_rotated = send_image_check_mediaquality(
            Viewtype::Image,
            Some("0"),
            bytes,
            "jpg",
            true, // has Exif
            2000,
            1800,
            270,
            1800,
            2000,
        )
        .await
        .unwrap();
        assert_correct_rotation(&img_rotated);

        let mut buf = Cursor::new(vec![]);
        img_rotated.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
        let bytes = buf.into_inner();

        let img_rotated = send_image_check_mediaquality(
            Viewtype::Image,
            Some("1"),
            &bytes,
            "jpg",
            false, // no Exif
            1800,
            2000,
            0,
            1800,
            2000,
        )
        .await
        .unwrap();
        assert_correct_rotation(&img_rotated);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_balanced_png() {
        let bytes = include_bytes!("../test-data/image/screenshot.png");

        send_image_check_mediaquality(
            Viewtype::Image,
            Some("0"),
            bytes,
            "png",
            false, // no Exif
            1920,
            1080,
            0,
            1920,
            1080,
        )
        .await
        .unwrap();

        send_image_check_mediaquality(
            Viewtype::Image,
            Some("1"),
            bytes,
            "png",
            false, // no Exif
            1920,
            1080,
            0,
            constants::WORSE_IMAGE_SIZE,
            constants::WORSE_IMAGE_SIZE * 1080 / 1920,
        )
        .await
        .unwrap();

        // This will be sent as Image, see [`BlobObject::maybe_sticker`] for explanation.
        send_image_check_mediaquality(
            Viewtype::Sticker,
            Some("0"),
            bytes,
            "png",
            false, // no Exif
            1920,
            1080,
            0,
            1920,
            1080,
        )
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_recode_image_huge_jpg() {
        let bytes = include_bytes!("../test-data/image/screenshot.jpg");
        send_image_check_mediaquality(
            Viewtype::Image,
            Some("0"),
            bytes,
            "jpg",
            true, // has Exif
            1920,
            1080,
            0,
            constants::BALANCED_IMAGE_SIZE,
            constants::BALANCED_IMAGE_SIZE * 1080 / 1920,
        )
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

    #[allow(clippy::too_many_arguments)]
    async fn send_image_check_mediaquality(
        viewtype: Viewtype,
        media_quality_config: Option<&str>,
        bytes: &[u8],
        extension: &str,
        has_exif: bool,
        original_width: u32,
        original_height: u32,
        orientation: i32,
        compressed_width: u32,
        compressed_height: u32,
    ) -> anyhow::Result<DynamicImage> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        alice
            .set_config(Config::MediaQuality, media_quality_config)
            .await?;
        let file = alice.get_blobdir().join("file").with_extension(extension);

        fs::write(&file, &bytes)
            .await
            .context("failed to write file")?;
        check_image_size(&file, original_width, original_height);

        let blob = BlobObject::new_from_path(&alice, &file).await?;
        let (_, exif) = blob.metadata()?;
        if has_exif {
            let exif = exif.unwrap();
            assert_eq!(exif_orientation(&exif, &alice), orientation);
        } else {
            assert!(exif.is_none());
        }

        let mut msg = Message::new(viewtype);
        msg.set_file(file.to_str().unwrap(), None);
        let chat = alice.create_chat(&bob).await;
        let sent = alice.send_msg(chat.id, &mut msg).await;
        let alice_msg = alice.get_last_msg().await;
        assert_eq!(alice_msg.get_width() as u32, compressed_width);
        assert_eq!(alice_msg.get_height() as u32, compressed_height);
        check_image_size(
            alice_msg.get_file(&alice).unwrap(),
            compressed_width,
            compressed_height,
        );

        let bob_msg = bob.recv_msg(&sent).await;
        assert_eq!(bob_msg.get_viewtype(), Viewtype::Image);
        assert_eq!(bob_msg.get_width() as u32, compressed_width);
        assert_eq!(bob_msg.get_height() as u32, compressed_height);
        let file = bob_msg.get_file(&bob).unwrap();

        let blob = BlobObject::new_from_path(&bob, &file).await?;
        let (_, exif) = blob.metadata()?;
        assert!(exif.is_none());

        let img = check_image_size(file, compressed_width, compressed_height);
        Ok(img)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_increation_in_blobdir() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "abc").await?;

        let file = t.get_blobdir().join("anyfile.dat");
        fs::write(&file, b"bla").await?;
        let mut msg = Message::new(Viewtype::File);
        msg.set_file(file.to_str().unwrap(), None);
        let prepared_id = chat::prepare_msg(&t, chat_id, &mut msg).await?;
        assert_eq!(prepared_id, msg.id);
        assert!(msg.is_increation());

        let msg = Message::load_from_db(&t, prepared_id).await?;
        assert!(msg.is_increation());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_increation_not_blobdir() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "abc").await?;
        assert_ne!(t.get_blobdir().to_str(), t.dir.path().to_str());

        let file = t.dir.path().join("anyfile.dat");
        fs::write(&file, b"bla").await?;
        let mut msg = Message::new(Viewtype::File);
        msg.set_file(file.to_str().unwrap(), None);
        assert!(chat::prepare_msg(&t, chat_id, &mut msg).await.is_err());

        Ok(())
    }
}
