extern crate irc;
extern crate libc;

use self::libc::{c_void};

#[inline(always)]
pub fn get_nick(msg_prefix: &Option<String>) -> Option<String> {
    let mut result = None;

    if let &Some(ref nickname) = msg_prefix {
        result = Some(nickname[..nickname.find('!').unwrap_or(0)].to_string());
    }

    result
}

extern {
    fn je_malloc_usable_size(ptr: *const c_void) -> u64;
}

///Calculates the size of memory which is allocated for pointer.
pub fn heap_size_of(ptr: *const c_void) -> usize {
    if ptr == 0x01 as *const c_void {
        0
    } else {
        unsafe { je_malloc_usable_size(ptr) as usize }
    }
}

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

