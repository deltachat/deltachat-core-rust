use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    /// extension.  The `data` will be written into the file.
    ///
    /// # Errors
    ///
    /// [BlobErrorKind::CreateFailure] is used when the file could not
    /// be created.  You can expect [BlobError.cause] to contain an
    /// underlying error.
    ///
    /// [BlobErrorKind::WriteFailure] is used when the file could not
    /// be written to.  You can expect [BlobError.cause] to contain an
    /// underlying error.
    pub fn create(
        context: &'a Context,
        suggested_name: impl AsRef<str>,
        data: &[u8],
    ) -> std::result::Result<BlobObject<'a>, BlobError> {
        let blobdir = context.get_blobdir();
        let (stem, ext) = BlobObject::sanitise_name(suggested_name.as_ref().to_string());
        let mut name = format!("{}{}", stem, ext);
        let max_attempt = 15;
        for attempt in 0..max_attempt {
            let path = blobdir.join(&name);
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)
            {
                Ok(mut file) => {
                    file.write_all(data)
                        .map_err(|err| BlobError::new_write_failure(blobdir, &name, err))?;
                    let blob = BlobObject {
                        blobdir,
                        name: format!("$BLOBDIR/{}", name),
                    };
                    context.call_cb(Event::NewBlobFile(blob.as_name().to_string()));
                    return Ok(blob);
                }
                Err(err) => {
                    if attempt == max_attempt {
                        return Err(BlobError::new_create_failure(blobdir, &name, err));
                    } else {
                        name = format!("{}-{}{}", stem, rand::random::<u32>(), ext);
                    }
                }
            }
        }
        Err(BlobError::new_create_failure(
            blobdir,
            &name,
            format_err!("Unreachable code - supposedly"),
        ))
    }

    /// Creates a new blob object with unique name by copying an existing file.
    ///
    /// This creates a new blob as described in [BlobObject::create]
    /// but also copies an existing file into it.
    ///
    /// # Errors
    ///
    /// In addition to the errors in [BlobObject::create] the
    /// [BlobErrorKind::CopyFailure] is used when the data can not be
    /// copied.
    pub fn create_and_copy(
        context: &'a Context,
        src: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject<'a>, BlobError> {
        let blob = BlobObject::create(context, src.as_ref().to_string_lossy(), b"")?;
        fs::copy(src.as_ref(), blob.to_abs_path()).map_err(|err| {
            fs::remove_file(blob.to_abs_path()).ok();
            BlobError::new_copy_failure(blob.blobdir, &blob.name, src.as_ref(), err)
        })?;
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
    pub fn create_from_path(
        context: &Context,
        src: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject, BlobError> {
        match src.as_ref().starts_with(context.get_blobdir()) {
            true => BlobObject::from_path(context, src),
            false => BlobObject::create_and_copy(context, src),
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
    /// [BlobErrorKind::WrongBlobdir] is used if the path is not in
    /// the blob directory.
    ///
    /// [BlobErrorKind::WrongName] is used if the file name does not
    /// remain identical after sanitisation.
    pub fn from_path(
        context: &Context,
        path: impl AsRef<Path>,
    ) -> std::result::Result<BlobObject, BlobError> {
        let rel_path = path
            .as_ref()
            .strip_prefix(context.get_blobdir())
            .map_err(|_| BlobError::new_wrong_blobdir(context.get_blobdir(), path.as_ref()))?;
        if !BlobObject::is_acceptible_blob_name(&rel_path) {
            return Err(BlobError::new_wrong_name(path.as_ref()));
        }
        let name = rel_path
            .to_str()
            .ok_or_else(|| BlobError::new_wrong_name(path.as_ref()))?;
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
    /// [BlobErrorKind::WrongName] is used if the name is not a valid
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
            return Err(BlobError::new_wrong_name(name));
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
    fn sanitise_name(mut name: String) -> (String, String) {
        for part in name.rsplit('/') {
            if part.len() > 0 {
                name = part.to_string();
                break;
            }
        }
        for part in name.rsplit('\\') {
            if part.len() > 0 {
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
        let mut iter = clean.rsplitn(2, '.');
        let mut ext = iter.next().unwrap_or_default().to_string();
        let mut stem = iter.next().unwrap_or_default().to_string();
        ext.truncate(32);
        stem.truncate(64);
        match stem.len() {
            0 => (ext, "".to_string()),
            _ => (stem, format!(".{}", ext).to_lowercase()),
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
}

impl<'a> fmt::Display for BlobObject<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$BLOBDIR/{}", self.name)
    }
}

/// Errors for the [BlobObject].
///
/// To keep the return type small and thus the happy path fast this
/// stores everything on the heap.
#[derive(Debug)]
pub struct BlobError {
    inner: Box<BlobErrorInner>,
}

#[derive(Debug)]
struct BlobErrorInner {
    kind: BlobErrorKind,
    data: BlobErrorData,
    backtrace: failure::Backtrace,
}

/// Error kind for [BlobError].
///
/// Each error kind has associated data in the [BlobErrorData].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobErrorKind {
    /// Failed to create the blob.
    CreateFailure,
    /// Failed to write data to blob.
    WriteFailure,
    /// Failed to copy data to blob.
    CopyFailure,
    /// Blob is not in the blobdir.
    WrongBlobdir,
    /// Blob has a bad name.
    ///
    /// E.g. the name is not sanitised correctly or contains a
    /// sub-directory.
    WrongName,
}

/// Associated data for each [BlobError] error kind.
///
/// This is not stored directly on the [BlobErrorKind] so that the
/// kind can stay trivially Copy and Eq.  It is however possible to
/// create a [BlobError] with mismatching [BlobErrorKind] and
/// [BlobErrorData], don't do that.
///
/// Any blobname stored here is the bare name, without the `$BLOBDIR`
/// prefix.  All data is owned so that errors do not need to be tied
/// to any lifetimes.
#[derive(Debug)]
enum BlobErrorData {
    CreateFailure {
        blobdir: PathBuf,
        blobname: String,
        cause: failure::Error,
    },
    WriteFailure {
        blobdir: PathBuf,
        blobname: String,
        cause: failure::Error,
    },
    CopyFailure {
        blobdir: PathBuf,
        blobname: String,
        src: PathBuf,
        cause: failure::Error,
    },
    WrongBlobdir {
        blobdir: PathBuf,
        src: PathBuf,
    },
    WrongName {
        blobname: PathBuf,
    },
}

impl BlobError {
    pub fn kind(&self) -> BlobErrorKind {
        self.inner.kind
    }

    fn new_create_failure(
        blobdir: impl Into<PathBuf>,
        blobname: impl Into<String>,
        cause: impl Into<failure::Error>,
    ) -> BlobError {
        BlobError {
            inner: Box::new(BlobErrorInner {
                kind: BlobErrorKind::CreateFailure,
                data: BlobErrorData::CreateFailure {
                    blobdir: blobdir.into(),
                    blobname: blobname.into(),
                    cause: cause.into(),
                },
                backtrace: failure::Backtrace::new(),
            }),
        }
    }

    fn new_write_failure(
        blobdir: impl Into<PathBuf>,
        blobname: impl Into<String>,
        cause: impl Into<failure::Error>,
    ) -> BlobError {
        BlobError {
            inner: Box::new(BlobErrorInner {
                kind: BlobErrorKind::WriteFailure,
                data: BlobErrorData::WriteFailure {
                    blobdir: blobdir.into(),
                    blobname: blobname.into(),
                    cause: cause.into(),
                },
                backtrace: failure::Backtrace::new(),
            }),
        }
    }

    fn new_copy_failure(
        blobdir: impl Into<PathBuf>,
        blobname: impl Into<String>,
        src: impl Into<PathBuf>,
        cause: impl Into<failure::Error>,
    ) -> BlobError {
        BlobError {
            inner: Box::new(BlobErrorInner {
                kind: BlobErrorKind::CopyFailure,
                data: BlobErrorData::CopyFailure {
                    blobdir: blobdir.into(),
                    blobname: blobname.into(),
                    src: src.into(),
                    cause: cause.into(),
                },
                backtrace: failure::Backtrace::new(),
            }),
        }
    }

    fn new_wrong_blobdir(blobdir: impl Into<PathBuf>, src: impl Into<PathBuf>) -> BlobError {
        BlobError {
            inner: Box::new(BlobErrorInner {
                kind: BlobErrorKind::WrongBlobdir,
                data: BlobErrorData::WrongBlobdir {
                    blobdir: blobdir.into(),
                    src: src.into(),
                },
                backtrace: failure::Backtrace::new(),
            }),
        }
    }

    fn new_wrong_name(blobname: impl Into<PathBuf>) -> BlobError {
        BlobError {
            inner: Box::new(BlobErrorInner {
                kind: BlobErrorKind::WrongName,
                data: BlobErrorData::WrongName {
                    blobname: blobname.into(),
                },
                backtrace: failure::Backtrace::new(),
            }),
        }
    }
}

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Match on the data rather than kind, they are equivalent for
        // identifying purposes but contain the actual data we need.
        match &self.inner.data {
            BlobErrorData::CreateFailure {
                blobdir, blobname, ..
            } => write!(
                f,
                "Failed to create blob {} in {}",
                blobname,
                blobdir.display()
            ),
            BlobErrorData::WriteFailure {
                blobdir, blobname, ..
            } => write!(
                f,
                "Failed to write data to blob {} in {}",
                blobname,
                blobdir.display()
            ),
            BlobErrorData::CopyFailure {
                blobdir,
                blobname,
                src,
                ..
            } => write!(
                f,
                "Failed to copy data from {} to blob {} in {}",
                src.display(),
                blobname,
                blobdir.display(),
            ),
            BlobErrorData::WrongBlobdir { blobdir, src } => write!(
                f,
                "File path {} is not in blobdir {}",
                src.display(),
                blobdir.display(),
            ),
            BlobErrorData::WrongName { blobname } => {
                write!(f, "Blob has a bad name: {}", blobname.display(),)
            }
        }
    }
}

impl failure::Fail for BlobError {
    fn cause(&self) -> Option<&dyn failure::Fail> {
        match &self.inner.data {
            BlobErrorData::CreateFailure { cause, .. }
            | BlobErrorData::WriteFailure { cause, .. }
            | BlobErrorData::CopyFailure { cause, .. } => Some(cause.as_fail()),
            _ => None,
        }
    }

    fn backtrace(&self) -> Option<&failure::Backtrace> {
        Some(&self.inner.backtrace)
    }
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
        let foo = BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        assert_eq!(foo.suffix(), Some("txt"));
        let bar = BlobObject::create(&t.ctx, "bar", b"world").unwrap();
        assert_eq!(bar.suffix(), None);
    }

    #[test]
    fn test_create_dup() {
        let t = dummy_context();
        BlobObject::create(&t.ctx, "foo.txt", b"hello").unwrap();
        let foo = t.ctx.get_blobdir().join("foo.txt");
        assert!(foo.exists());
        BlobObject::create(&t.ctx, "foo.txt", b"world").unwrap();
        for dirent in fs::read_dir(t.ctx.get_blobdir()).unwrap() {
            let fname = dirent.unwrap().file_name();
            if fname == foo.file_name().unwrap() {
                assert_eq!(fs::read(&foo).unwrap(), b"hello");
            } else {
                let name = fname.to_str().unwrap();
                assert!(name.starts_with("foo"));
                assert!(name.ends_with(".txt"));
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
        let blob = BlobObject::create_from_path(&t.ctx, &src_ext).unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/external");
        let data = fs::read(blob.to_abs_path()).unwrap();
        assert_eq!(data, b"boo");

        let src_int = t.ctx.get_blobdir().join("internal");
        fs::write(&src_int, b"boo").unwrap();
        let blob = BlobObject::create_from_path(&t.ctx, &src_int).unwrap();
        assert_eq!(blob.as_name(), "$BLOBDIR/internal");
        let data = fs::read(blob.to_abs_path()).unwrap();
        assert_eq!(data, b"boo");
    }
    #[test]
    fn test_create_from_name_long() {
        let t = dummy_context();
        let src_ext = t.dir.path().join("autocrypt-setup-message-4137848473.html");
        fs::write(&src_ext, b"boo").unwrap();
        let blob = BlobObject::create_from_path(&t.ctx, &src_ext).unwrap();
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
}
