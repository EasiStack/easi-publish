use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use chrono::Datelike;
use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, Source, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Features, Library, LibraryExt};

use crate::clock::Clock;
use crate::data::DataSet;
use crate::error::{PublishError, Result};
use crate::fonts::SharedFonts;
use crate::template::project_file_id;

pub(crate) struct PublishWorld {
    root: Option<PathBuf>,
    main_source: Source,
    library: LazyHash<Library>,
    fonts: Arc<typst_kit::fonts::FontStore>,
    virtual_files: HashMap<FileId, Bytes>,
    clock: Clock,
    // Memoized wall-clock instant for `today()`. Captured on the first call so
    // repeated `datetime.today()` invocations within a single render agree even
    // across a second/day boundary (mirrors typst-kit's `datetime::Time`).
    // Unused for `Clock::Fixed`.
    now: OnceLock<chrono::DateTime<chrono::Utc>>,
}

impl PublishWorld {
    pub fn new(
        root: Option<PathBuf>,
        main_source: Source,
        shared_fonts: &SharedFonts,
        data: DataSet,
        clock: Clock,
        features: Features,
    ) -> Result<Self> {
        // `features` selects optional Typst capabilities. The PDF path passes
        // `Features::default()`. The HTML path enables `Feature::Html` so the
        // `html` module (`html.elem`, ...) is defined for templates.
        let mut builder = Library::builder().with_features(features);
        
        // `sys.inputs` (independent of any virtual files).
        if let Some(dict) = data.inputs {
            builder = builder.with_inputs(dict);
        }
        let library = builder.build();

        // Virtual files, each mapped through `project_file_id` (the only path to
        // a `FileId`, which keeps names confined to the project root). Reject a
        // name that collides with the template's own file, which would otherwise
        // silently shadow it. Duplicate names among the files were already
        // rejected when the `DataSet` was built.
        let main_id = main_source.id();
        let mut virtual_files = HashMap::with_capacity(data.files.len());
        for (name, bytes) in data.files {
            let file_id = project_file_id(name.as_str());
            if file_id == main_id {
                return Err(PublishError::InvalidTemplate(format!(
                    "virtual file '{name}' collides with the template's file name"
                )));
            }
            virtual_files.insert(file_id, Bytes::new(bytes));
        }

        Ok(Self {
            root,
            main_source,
            library: LazyHash::new(library),
            fonts: Arc::clone(&shared_fonts.store),
            virtual_files,
            clock,
            now: OnceLock::new(),
        })
    }

    fn resolve_path(&self, id: FileId) -> FileResult<PathBuf> {
        // Package imports (`@preview/...`) are not supported, we never fetch
        // or resolve them. Deny explicitly rather than silently mis-resolving
        // the in-package path against the local root.
        if matches!(id.root(), VirtualRoot::Package(_)) {
            return Err(FileError::Package(PackageError::Other(Some(
                "package imports are not supported".into(),
            ))));
        }
        let root = self.root.as_ref().ok_or(FileError::AccessDenied)?;
        let vpath = id.vpath();
        
        // `realize` confines the virtual path to `root` and rejects traversal out of it.
        // Map any failure to `AccessDenied` to preserve the prior contract.
        vpath.realize(root).map_err(|_| FileError::AccessDenied)
    }
}

impl typst::World for PublishWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main_source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_source.id() {
            return Ok(self.main_source.clone());
        }
        let path = self.resolve_path(id)?;
        let text = std::fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Source::new(id, text))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.virtual_files.get(&id) {
            return Ok(bytes.clone());
        }
        let path = self.resolve_path(id)?;
        let raw = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Bytes::new(raw))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        // `offset` is passed by the template via `datetime.today(offset: d)`. As
        // of typst 0.15 it is a `Duration` (allowing sub-hour precision) rather
        // than a whole-hour integer. `None` means the template asked for `auto`.
        //
        // Key point: an explicit offset is *absolute* (UTC + d), not "add d to the
        // local time". So once a real offset is given, it no longer matters whether
        // the clock is Utc or SystemLocal, the answer is UTC+d either way, which is
        // why those two arms for `now` below have the same body. The clock only changes
        // what `None` means, which is Utc -> UTC+0 or SystemLocal -> the host's local time.

        // Fixed always returns the same date, so it deliberately ignores `offset`,
        // which is why it is handled here first up front.
        if let Clock::Fixed { year, month, day } = self.clock {
            return Datetime::from_ymd(year, month, day);
        }

        // A Typst `Duration` reports its *total* length in seconds, which is
        // exactly the eastward UTC offset we want to hand chrono.
        let to_fixed = |d: Duration| -> Option<chrono::FixedOffset> {
            let secs = i32::try_from(d.seconds() as i64).ok()?;
            chrono::FixedOffset::east_opt(secs)
        };

        // Capture "now" once per render and reuse it for every `today()` call.
        let now_utc = *self.now.get_or_init(chrono::Utc::now);

        // Handle clocks based on the current time.
        let now = match (self.clock, offset) {
            // Utc: shift "now" by the requested offset, or UTC+0 if none was given.
            (Clock::Utc, off) => {
                let fixed = match off {
                    Some(d) => to_fixed(d)?,
                    None => chrono::FixedOffset::east_opt(0)?,
                };
                now_utc.with_timezone(&fixed).naive_local()
            }
            // SystemLocal but with an explicit offset. It's absolute, so we shift
            // UTC by it and ignore the host's local zone (same result as Utc above).
            (Clock::SystemLocal, Some(d)) => {
                now_utc.with_timezone(&to_fixed(d)?).naive_local()
            }
            // SystemLocal with `auto`. Just use the host's local wall clock.
            (Clock::SystemLocal, None) => now_utc.with_timezone(&chrono::Local).naive_local(),
            (Clock::Fixed { .. }, _) => unreachable!("handled above"),
        };

        Datetime::from_ymd(
            now.year(),
            now.month().try_into().ok()?,
            now.day().try_into().ok()?,
        )
    }
}

// Compile-time assertion that PublishWorld is Send + Sync (required by World trait).
const _: () = {
    fn _assert<T: Send + Sync>() {}
    #[allow(unused)]
    fn _check() {
        _assert::<PublishWorld>();
    }
};
