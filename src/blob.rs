//! # Blob directory management

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use image::GenericImageView;
use thiserror::Error;

use crate::constants::AVATAR_SIZE;
use crate::context::Context;
use crate::events::Event;

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
    ///
    /// # Errors
    ///
    /// [BlobError::CreateFailure] is used when the file could not
    /// be created.  You can expect [BlobError.cause] to contain an
    /// underlying error.
    ///
    /// [BlobError::WriteFailure] is used when the file could not
    /// be written to.  You can expect [BlobError.cause] to contain an
    /// underlying error.
    pub fn create(
        context: &'a Context,
        suggested_name: impl AsRef<str>,
        data: &[u8],
    ) -> std::result::Result<BlobObject<'a>, BlobError> {
        let blobdir = context.get_blobdir();
        let (stem, ext) = BlobObject::sanitise_name(suggested_name.as_ref());
        let (name, mut file) = BlobObject::create_new_file(&blobdir, &stem, &ext)?;
        file.write_all(data)
            .map_err(|err| BlobError::WriteFailure {
                blobdir: blobdir.to_path_buf(),
                blobname: name.clone(),
                cause: err,
            })?;
        let blob = BlobObject {
            blobdir,
            name: format!("$BLOBDIR/{}", name),
        };
        context.call_cb(Event::NewBlobFile(blob.as_name().to_string()));
        Ok(blob)
    }

    // Creates a new file, returning a tuple of the name and the handle.
    fn create_new_file(dir: &Path, stem: &str, ext: &str) -> Result<(String, fs::File), BlobError> {
        let max_attempt = 15;
        let mut name = format!("{}{}", stem, ext);
        for attempt in 0..max_attempt {
            let path = dir.join(&name);
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
            {
                Ok(file) => return Ok((name, file)),
                Err(err) => {
                    if attempt == max_attempt {
                        return Err(BlobError::CreateFailure {
                            blobdir: dir.to_path_buf(),
                            blobname: name,
                            cause: err,
                        });
                    } else {
                        name = format!("{}-{}{}", stem, rand::random::<u32>(), ext);
                    }
                }
            }
        }
        // This is supposed to be unreachable, but the compiler doesn't know.
        Err(BlobError::CreateFailure {
            blobdir: dir.to_path_buf(),
            blobname: name,
            cause: std::io::Error::new(std::io::ErrorKind::Other, "supposedly unreachable"),
        })
    }

    /// Creates a new blob object with unique name by copying an existing file.
    ///
    /// This creates a new blob as described in [BlobObject::create]
    /// but also copies an existing file into it.  This is done in a
    /// in way which avoids race-conditions when multiple files are
    /// concurrently created.
    ///
    /// # Errors
    ///
    /// In addition to the errors in [BlobObject::create] the
    /// [BlobError::CopyFailure] is used when the data can not be
    /// copied.
    pub fn create_and_copy(
        context: &'a Context,
        src: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject<'a>, BlobError> {
        let mut src_file = fs::File::open(src.as_ref()).map_err(|err| BlobError::CopyFailure {
            blobdir: context.get_blobdir().to_path_buf(),
            blobname: String::from(""),
            src: src.as_ref().to_path_buf(),
            cause: err,
        })?;
        let (stem, ext) = BlobObject::sanitise_name(&src.as_ref().to_string_lossy());
        let (name, mut dst_file) = BlobObject::create_new_file(context.get_blobdir(), &stem, &ext)?;
        let name_for_err = name.clone();
        std::io::copy(&mut src_file, &mut dst_file).map_err(|err| {
            {
                // Attempt to remove the failed file, swallow errors resulting from that.
                let path = context.get_blobdir().join(&name_for_err);
                fs::remove_file(path).ok();
            }
            BlobError::CopyFailure {
                blobdir: context.get_blobdir().to_path_buf(),
                blobname: name_for_err,
                src: src.as_ref().to_path_buf(),
                cause: err,
            }
        })?;
        let blob = BlobObject {
            blobdir: context.get_blobdir(),
            name: format!("$BLOBDIR/{}", name),
        };
        context.call_cb(Event::NewBlobFile(blob.as_name().to_string()));
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
    /// # Errors
    ///
    /// This merely delegates to the [BlobObject::create_and_copy] and
    /// the [BlobObject::from_path] methods.  See those for possible
    /// errors.
    pub fn new_from_path(
        context: &Context,
        src: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject, BlobError> {
        if src.as_ref().starts_with(context.get_blobdir()) {
            BlobObject::from_path(context, src)
        } else {
            BlobObject::create_and_copy(context, src)
        }
    }

    /// Returns a [BlobObject] for an existing blob from a path.
    ///
    /// The path must designate a file directly in the blobdir and
    /// must use a valid blob name.  That is after sanitisation the
    /// name must still be the same, that means it must be valid UTF-8
    /// and not have any special characters in it.
    ///
    /// # Errors
    ///
    /// [BlobError::WrongBlobdir] is used if the path is not in
    /// the blob directory.
    ///
    /// [BlobError::WrongName] is used if the file name does not
    /// remain identical after sanitisation.
    pub fn from_path(
        context: &Context,
        path: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject, BlobError> {
        let rel_path = path
            .as_ref()
            .strip_prefix(context.get_blobdir())
            .map_err(|_| BlobError::WrongBlobdir {
                blobdir: context.get_blobdir().to_path_buf(),
                src: path.as_ref().to_path_buf(),
            })?;
        if !BlobObject::is_acceptible_blob_name(&rel_path) {
            return Err(BlobError::WrongName {
                blobname: path.as_ref().to_path_buf(),
            });
        }
        let name = rel_path.to_str().ok_or_else(|| BlobError::WrongName {
            blobname: path.as_ref().to_path_buf(),
        })?;
        BlobObject::from_name(context, name.to_string())
    }

    /// Returns a [BlobObject] for an existing blob.
    ///
    /// The `name` may optionally be prefixed with the `$BLOBDIR/`
    /// prefixed, as returned by [BlobObject::as_name].  This is how
    /// you want to create a [BlobObject] for a filename read from the
    /// database.
    ///
    /// # Errors
    ///
    /// [BlobError::WrongName] is used if the name is not a valid
    /// blobname, i.e. if [BlobObject::sanitise_name] does modify the
    /// provided name.
    pub fn from_name(
        context: &'a Context,
        name: String,
    ) -> std::result::Result<BlobObject<'a>, BlobError> {
        let name: String = match name.starts_with("$BLOBDIR/") {
            true => name.splitn(2, '/').last().unwrap().to_string(),
            false => name,
        };
        if !BlobObject::is_acceptible_blob_name(&name) {
            return Err(BlobError::WrongName {
                blobname: PathBuf::from(name),
            });
        }
        Ok(BlobObject {
            blobdir: context.get_blobdir(),
            name: format!("$BLOBDIR/{}", name),
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
        self.name.rsplitn(2, '/').next().unwrap()
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
        let ext = self.name.rsplitn(2, '.').next();
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
        let mut iter = clean.splitn(2, '.');
        let stem: String = iter.next().unwrap_or_default().chars().take(64).collect();
        let ext: String = iter.next().unwrap_or_default().chars().take(32).collect();
        if ext.is_empty() {
            (stem, "".to_string())
        } else {
            (stem, format!(".{}", ext).to_lowercase())
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

    pub fn recode_to_avatar_size(&self, context: &Context) -> Result<(), BlobError> {
        let blob_abs = self.to_abs_path();
        let img = image::open(&blob_abs).map_err(|err| BlobError::RecodeFailure {
            blobdir: context.get_blobdir().to_path_buf(),
            blobname: blob_abs.to_str().unwrap_or_default().to_string(),
            cause: err,
        })?;

        if img.width() <= AVATAR_SIZE && img.height() <= AVATAR_SIZE {
            return Ok(());
        }

        let img = img.thumbnail(AVATAR_SIZE, AVATAR_SIZE);

        img.save(&blob_abs).map_err(|err| BlobError::WriteFailure {
            blobdir: context.get_blobdir().to_path_buf(),
            blobname: blob_abs.to_str().unwrap_or_default().to_string(),
            cause: err,
        })?;

        Ok(())
    }
}

impl<'a> fmt::Display for BlobObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$BLOBDIR/{}", self.name)
    }
}

/// Errors for the [BlobObject].
#[derive(Debug, Error)]
pub enum BlobError {
    #[error("Failed to create blob {blobname} in {}", .blobdir.display())]
    CreateFailure {
        blobdir: PathBuf,
        blobname: String,
        #[source]
        cause: std::io::Error,
    },
    #[error("Failed to write data to blob {blobname} in {}", .blobdir.display())]
    WriteFailure {
        blobdir: PathBuf,
        blobname: String,
        #[source]
        cause: std::io::Error,
    },
    #[error("Failed to copy data from {} to blob {blobname} in {}", .src.display(), .blobdir.display())]
    CopyFailure {
        blobdir: PathBuf,
        blobname: String,
        src: PathBuf,
        #[source]
        cause: std::io::Error,
    },
    #[error("Failed to recode to blob {blobname} in {}", .blobdir.display())]
    RecodeFailure {
        blobdir: PathBuf,
        blobname: String,
        #[source]
        cause: image::ImageError,
    },
    #[error("File path {} is not in {}", .src.display(), .blobdir.display())]
    WrongBlobdir { blobdir: PathBuf, src: PathBuf },
    #[error("Blob has a badname {}", .blobname.display())]
    WrongName { blobname: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[test]
    fn test_create() {
        let t = dummy_context();
        let blob = BlobObject::create(&t.ctx, "foo", b"hello").unwrap();
        let fname = t.ctx.get_blobdir().join("foo");
        let data = fs::read(fname).unwrap();
        assert_eq!(data, b"hello");
        assert_eq!(blob.as_name(), "$BLOBDIR/foo");
        assert_eq!(blob.to_abs_path(), t.ctx.get_blobdir().join("foo"));
    }

    #[test]
    fn test_lowercase_ext() {
        let t = dummy_context();
        let blob = BlobObject::create(&t.ctx, "foo.TXT", b"hello").unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/foo.txt");
    }

    #[test]
    fn test_as_file_name() {
        let t = dummy_context();
        let blob = BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        assert_eq!(blob.as_file_name(), "foo.txt");
    }

    #[test]
    fn test_as_rel_path() {
        let t = dummy_context();
        let blob = BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        assert_eq!(blob.as_rel_path(), Path::new("foo.txt"));
    }

    #[test]
    fn test_suffix() {
        let t = dummy_context();
        let blob = BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        assert_eq!(blob.suffix(), Some("txt"));
        let blob = BlobObject::create(&t.ctx, "bar", b"world").unwrap();
        assert_eq!(blob.suffix(), None);
    }

    #[test]
    fn test_create_dup() {
        let t = dummy_context();
        BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        let foo_path = t.ctx.get_blobdir().join("foo.txt");
        assert!(foo_path.exists());
        BlobObject::create(&t.ctx, "foo.txt", b"world").unwrap();
        for dirent in fs::read_dir(t.ctx.get_blobdir()).unwrap() {
            let fname = dirent.unwrap().file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).unwrap(), b"hello");
            } else {
                let name = fname.to_str().unwrap();
                assert!(name.starts_with("foo"));
                assert!(name.ends_with(".txt"));
            }
        }
    }

    #[test]
    fn test_double_ext_preserved() {
        let t = dummy_context();
        BlobObject::create(&t.ctx, "foo.tar.gz", b"hello").unwrap();
        let foo_path = t.ctx.get_blobdir().join("foo.tar.gz");
        assert!(foo_path.exists());
        BlobObject::create(&t.ctx, "foo.tar.gz", b"world").unwrap();
        for dirent in fs::read_dir(t.ctx.get_blobdir()).unwrap() {
            let fname = dirent.unwrap().file_name();
            if fname == foo_path.file_name().unwrap() {
                assert_eq!(fs::read(&foo_path).unwrap(), b"hello");
            } else {
                let name = fname.to_str().unwrap();
                println!("{}", name);
                assert!(name.starts_with("foo"));
                assert!(name.ends_with(".tar.gz"));
            }
        }
    }

    #[test]
    fn test_create_long_names() {
        let t = dummy_context();
        let s = "1".repeat(150);
        let blob = BlobObject::create(&t.ctx, &s, b"data").unwrap();
        let blobname = blob.as_name().split('/').last().unwrap();
        assert!(blobname.len() < 128);
    }

    #[test]
    fn test_create_and_copy() {
        let t = dummy_context();
        let src = t.dir.path().join("src");
        fs::write(&src, b"boo").unwrap();
        let blob = BlobObject::create_and_copy(&t.ctx, &src).unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/src");
        let data = fs::read(blob.to_abs_path()).unwrap();
        assert_eq!(data, b"boo");

        let whoops = t.dir.path().join("whoops");
        assert!(BlobObject::create_and_copy(&t.ctx, &whoops).is_err());
        let whoops = t.ctx.get_blobdir().join("whoops");
        assert!(!whoops.exists());
    }

    #[test]
    fn test_create_from_path() {
        let t = dummy_context();

        let src_ext = t.dir.path().join("external");
        fs::write(&src_ext, b"boo").unwrap();
        let blob = BlobObject::new_from_path(&t.ctx, &src_ext).unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/external");
        let data = fs::read(blob.to_abs_path()).unwrap();
        assert_eq!(data, b"boo");

        let src_int = t.ctx.get_blobdir().join("internal");
        fs::write(&src_int, b"boo").unwrap();
        let blob = BlobObject::new_from_path(&t.ctx, &src_int).unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/internal");
        let data = fs::read(blob.to_abs_path()).unwrap();
        assert_eq!(data, b"boo");
    }
    #[test]
    fn test_create_from_name_long() {
        let t = dummy_context();
        let src_ext = t.dir.path().join("autocrypt-setup-message-4137848473.html");
        fs::write(&src_ext, b"boo").unwrap();
        let blob = BlobObject::new_from_path(&t.ctx, &src_ext).unwrap();
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
        let (_, ext) =
            BlobObject::sanitise_name("Я ЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯЯ.txt");
        assert_eq!(ext, ".txt");
    }
}
