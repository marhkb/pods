// Copyright (c) 2023-2026 Sophie Herold
// Copyright (c) 2026 Marcus Behrendt
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;

pub(crate) use gettextrs::gettext;

fn freplace(s: String, args: impl IntoIterator<Item = impl Display>) -> String {
    args.into_iter()
        .fold(s, |s, arg| s.replacen("{}", &arg.to_string(), 1))
}

pub(crate) fn gettext_f(format: &str, args: impl IntoIterator<Item = impl Display>) -> String {
    freplace(ugettext(format), args)
}

pub(crate) fn ngettext(format: &str, format_plural: &str, n: u32) -> String {
    freplace(
        apply_unicode_escapes(gettextrs::ngettext(format, format_plural, n)),
        [n],
    )
}

pub(crate) fn ngettext_f(
    format: &str,
    format_plural: &str,
    n: u32,
    args: impl IntoIterator<Item = impl Display>,
) -> String {
    freplace(
        apply_unicode_escapes(gettextrs::ngettext(format, format_plural, n)),
        args,
    )
}

/// Same as `gettext` but evaluates unicode escape sequences
fn ugettext(msgid: &str) -> String {
    apply_unicode_escapes(gettext(msgid))
}

/// Replace unicode escape sequences with the actual `char`
fn apply_unicode_escapes(s: String) -> String {
    if !s.contains(r"\u{") {
        return s;
    }

    let mut new = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some((hex, rest)) = chars
                .as_str()
                .strip_prefix("u{")
                .and_then(|rest| rest.split_once('}'))
        {
            let Some(ch) = hex_to_char(hex) else {
                return s;
            };
            new.push(ch);
            chars = rest.chars();
        } else {
            new.push(c);
        }
    }

    new
}

/// Convert hex string to char
fn hex_to_char(hex: &str) -> Option<char> {
    let u = u32::from_str_radix(hex, 16).ok()?;
    char::from_u32(u)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_apply_unicode_escapes() {
        assert_eq!(
            apply_unicode_escapes(r"abc \u{03a6} \u{2764} d".to_owned()),
            "abc \u{03a6} \u{2764} d".to_owned()
        );
        assert_eq!(
            apply_unicode_escapes(r"C:\Users".to_owned()),
            r"C:\Users".to_owned()
        );
        assert_eq!(
            apply_unicode_escapes(r"C:\Users \u{03a6}".to_owned()),
            "C:\\Users \u{03a6}".to_owned()
        );
        assert_eq!(
            apply_unicode_escapes(r"a \u{zz} b".to_owned()),
            r"a \u{zz} b".to_owned()
        );
        assert_eq!(
            apply_unicode_escapes(r"a \u{03a6".to_owned()),
            r"a \u{03a6".to_owned()
        );
    }

    #[test]
    fn test_hex_to_char() {
        assert_eq!(hex_to_char("03a6"), Some('Φ'));
    }
}
