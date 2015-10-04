macro_rules! impl_is_text_checker {
    ($name:ident, $tp:ident, $($arg:pat),+) => {
        pub fn $name<T: AsRef<str>>(text: T) -> bool {
            let text = text.as_ref();
            text.chars().$tp(|elem_char| match elem_char { $($arg => true,)+
                                                           _ => false })
        }
    }
}

//impl_is_text_checker!(is_jp, any, '\u{3000}'...'\u{303f}',
//                                  '\u{3040}'...'\u{309f}',
//                                  '\u{30a0}'...'\u{30ff}',
//                                  '\u{ff00}'...'\u{ffef}',
//                                  '\u{4e00}'...'\u{9faf}',
//                                  '\u{3400}'...'\u{4dbf}');
//

