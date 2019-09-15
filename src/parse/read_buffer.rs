use super::file_location::*;

use std::collections::{VecDeque};

///
/// A character read buffer used by the tokenizer
///
pub struct TokenReadBuffer<Chars: Iterator<Item=char>> {
    /// Remaining characters to read
    chars: Chars,

    /// Characters that have been pushed back into this buffer
    pushed_back: VecDeque<char>,

    /// Characters that have been read from this buffer
    read: VecDeque<char>
}

impl<Chars: Iterator<Item=char>> TokenReadBuffer<Chars> {
    ///
    /// Creates a new character buffer for reading tokens from
    /// 
    pub fn new(read_from: Chars) -> TokenReadBuffer<Chars> {
        TokenReadBuffer {
            chars:          read_from,
            pushed_back:    VecDeque::new(),
            read:           VecDeque::new()
        }
    }

    ///
    /// Reads the next character if it's available
    ///
    pub fn read_next(&mut self) -> Option<char> {
        // Read the next character
        let next_chr = self.pushed_back.pop_back().or_else(|| self.chars.next());

        // Add to the pending 'read characters' buffer
        if let Some(next_chr) = next_chr {
            self.read.push_back(next_chr);
        }

        next_chr
    }

    /// 
    /// Puts the last character read back 
    /// 
    pub fn push_back(&mut self) {
        let last_chr = self.read.pop_back().expect("Cannot push_back when no read characters are pending");
        self.pushed_back.push_back(last_chr);
    }

    ///
    /// Clears the read characters and updates a file location
    /// 
    /// (File location is always cleared to make it impossible to update it from the same set of read characters twice)
    ///
    pub fn update_location(&mut self, last_location: FileLocation) -> FileLocation {
        last_location.update_from(self.read.drain(..))
    }

    ///
    /// Turns the current set of read characters into a string
    ///
    pub fn read_characters(&self) -> String {
        self.read.iter().map(|chr| *chr).collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_characters() {
        let mut buf = TokenReadBuffer::new("test".chars());

        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == Some('e'));
        assert!(buf.read_next() == Some('s'));
        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == None);
    }

    #[test]
    fn push_back_and_reread_character() {
        let mut buf = TokenReadBuffer::new("test".chars());

        assert!(buf.read_next() == Some('t'));
        buf.push_back();
        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == Some('e'));
    }

    #[test]
    fn read_character_string() {
        let mut buf = TokenReadBuffer::new("test".chars());

        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == Some('e'));
        assert!(buf.read_next() == Some('s'));
        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == None);

        assert!(buf.read_characters() == String::from("test"));
    }

    #[test]
    fn push_back_removes_from_character_string() {
        let mut buf = TokenReadBuffer::new("test".chars());

        assert!(buf.read_next() == Some('t'));
        assert!(buf.read_next() == Some('e'));
        assert!(buf.read_next() == Some('s'));
        assert!(buf.read_next() == Some('t'));

        buf.push_back();
        assert!(buf.read_characters() == String::from("tes"));

        buf.push_back();
        assert!(buf.read_characters() == String::from("te"));

        buf.push_back();
        assert!(buf.read_characters() == String::from("t"));

        buf.read_next();
        assert!(buf.read_characters() == String::from("te"));
        
        buf.read_next();
        assert!(buf.read_characters() == String::from("tes"));
    }
}
