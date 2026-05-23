#[macro_export]
macro_rules! logged {
    ($val:expr, $target:expr) => {
        match $val {
            Err(err) => {
                log::error!(target: $target, "{err}");
                Err(err)
            },
            Ok(val) => Ok(val),
        }
    };
    ($val:expr) => {
        logged!($val, module_path!())
    }
}
