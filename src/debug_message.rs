// If you want to use the macro under a specific module namespace, you can skip the
// #[macro_export]
macro_rules! print_debug_message {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}
//If you want to use the macro in any module without the declaration below and then refer to it with use debug_message::print_debug_message, you can use the #[macro_export] attribute
pub(crate) use print_debug_message;