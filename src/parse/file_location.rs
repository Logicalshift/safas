use std::sync::*;
use std::str::{Chars};

///
/// Represents a location in a file
///
#[derive(Clone, Debug)]
pub struct FileLocation {
    pub filename:   Arc<String>,
    pub line:       u32,
    pub chr_idx:    u32,
    pub file_idx:   u32
}

impl FileLocation {
    ///
    /// Creates a location at the start of the file with the specified name
    ///
    pub fn new(filename: &str) -> FileLocation {
        FileLocation {
            filename:   Arc::new(String::from(filename)),
            line:       1,
            chr_idx:    1,
            file_idx:   0
        }
    }

    ///
    /// Updates a file location from the specified set of characters
    ///
    pub fn update_from(self, characters: Chars) -> FileLocation {
        let mut result      = self;
        let mut last_chr    = ' ';

        for chr in characters {
            // Move along one character index
            result.chr_idx    += 1;
            result.file_idx   += 1;

            match chr {
                '\n'    => {
                    if last_chr != '\r' {
                        // '\n' on its own
                        result.line     += 1;
                        result.chr_idx  = 1;
                    } else {
                        // '\r\n' - DOS line ending
                        result.chr_idx  -= 1;
                    }
                },

                '\r'    => {
                    result.line     += 1;
                    result.chr_idx  = 1;
                }

                _ => {}
            }

            last_chr = chr;
        }

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn count_lines_from_one() {
        let location = FileLocation::new("test");
        assert!(location.line == 1);
    }

    #[test]
    fn newline_advances_line_number() {
        let location = FileLocation::new("test");
        let location = location.update_from("Test\n\nAnotherTest\n12345".chars());

        assert!(location.line == 4);
        assert!(location.chr_idx == 6);
    }

    #[test]
    fn file_index_counts_all_characters() {
        let location = FileLocation::new("test");
        let location = location.update_from("Test\n\nAnotherTest\n12345".chars());

        assert!(location.file_idx == 23);
    }

    #[test]
    fn dos_newline_only_advances_line_once() {
        let location = FileLocation::new("test");
        let location = location.update_from("Test\r\n12345".chars());

        assert!(location.line == 2);
        assert!(location.chr_idx == 6);
    }
}
