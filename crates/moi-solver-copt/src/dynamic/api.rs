use libloading::Library;

/// Loaded COPT C API function table.
///
/// Add typed function pointers here after `bindings::gen80` is generated.
#[derive(Debug)]
pub struct CoptApi {
    _library: Library,
}
