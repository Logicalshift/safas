use std::fmt::{Write};

///
/// Generate a hex-dump of byte data
///
pub fn hexdump(data: &Vec<u8>) -> String {
    let mut result = String::new();

    // Each row is 16 bytes
    let row_len     = 16;
    let num_rows    = (data.len()/row_len)+1;

    for row in 0..num_rows {
        // Start each row with a newline, except the first one
        if row != 0 {
            result += "\n";
        }

        // Each row starts with an address
        let row_address = row * row_len;
        write!(&mut result, "{:08x}: ", row_address).ok();

        // Then our 16 bytes, in groups of 4
        for byte in 0..row_len {
            // Separate into groups of 4
            if byte != 0 && (byte % 4) == 0 {
                result += " ";
            }

            if row_address + byte < data.len() {
                // Byte in string
                write!(&mut result, "{:02x}", data[row_address + byte]).ok();
            } else {
                // Missing byte
                result += "  ";
            }
        }

        // Then the summary
        result += " | ";

        for byte in 0..row_len {
            if row_address + byte < data.len() {
                let byte = data[row_address + byte];
                if byte < 32 || byte == 127 {
                    result += ".";
                } else {
                    result.push(char::from(byte));
                }
            }
        }
    }

    result
}
