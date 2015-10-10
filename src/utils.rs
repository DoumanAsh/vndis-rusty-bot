extern crate irc;
extern crate libc;

use self::libc::{c_void};
use std::fmt;
use std::fmt::Write;

pub struct Escape(pub String);

impl fmt::Display for Escape {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for elem in self.0.chars() {
            let norm = elem.to_string();
            try!(f.write_str(match elem {
                '"'     => "\\\"",
                '\\'    => "\\\\",
                '\x08'  => "\\b",
                '\x0c'  => "\\f",
                '\n'    => "\\n",
                '\r'    => "\\r",
                '\t'    => "\\t",
                _       => &norm,
            }));
        }

        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_nick() {
        let result = super::get_nick(&Some("KuuRusty!KuuRusty@irc.net".to_string()));

        assert!(result.is_some());
        assert!(result.unwrap() == "KuuRusty");

        let result = super::get_nick(&None);
        assert!(!result.is_some());
    }
}
