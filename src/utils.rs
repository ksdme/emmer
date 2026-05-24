#[macro_export]
macro_rules! logged {
    ($val:expr, $target:expr) => {
        $val.inspect_err(|err| log::error!(target: $target, "{err:?}"))
    };
    ($val:expr) => {
        logged!($val, module_path!())
    }
}
