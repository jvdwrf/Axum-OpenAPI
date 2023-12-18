macro_rules! err {
    (
        $span:expr, $msg:literal $(,$arg:expr)* $(,)?
    ) => {
        syn::Error::new(
            $span.span(),
            &format!($msg, $($arg),*)
        )
    };
    (
        $span:expr, $msg:expr
    ) => {
        syn::Error::new(
            $span.span(),
            $msg
        )
    };
}

// macro_rules! err_span {
//     (
//         $span:expr, $msg:literal $(,$arg:expr)* $(,)?
//     ) => {
//         syn::Error::new(
//             $span,
//             &format!($msg, $($arg),*)
//         )
//     };
//     (
//         $span:expr, $msg:expr
//     ) => {
//         syn::Error::new(
//             $span,
//             $msg
//         )
//     };
// }

macro_rules! err_call_site {
    (
        $msg:literal $(,$arg:expr)* $(,)?
    ) => {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            &format!($msg, $($arg),*)
        )
    };
    (
        $msg:expr
    ) => {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            $msg
        )
    };
}
