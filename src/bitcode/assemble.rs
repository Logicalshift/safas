use super::code::*;
use super::label::*;
use super::bitcode_monad::*;

use crate::meta::*;
use crate::exec::*;

use std::mem;
use std::collections::{HashMap, HashSet};

/// The maximum number of assembly passes we should attempt before deciding that a bitcode monad cannot be evaluated
const DEFAULT_MAX_PASSES: usize = 1000;

///
/// Represents an assembler that is running
///
struct Assembler {
    /// Known values for the labels
    label_values: HashMap<Label, CellRef>,

    /// Labels that have changed this pass
    changed_labels: HashSet<Label>,

    /// Bitcode that has been generated
    bitcode: Vec<BitCode>,

    /// The current bitcode position
    bit_pos: u64,

    /// The current offset, added to bit_pos when generating label values
    bit_offset: i64,

    /// The maximum number of passes we should attempt
    max_passes: usize
}

impl Assembler {
    ///
    /// Creates a new assembler
    ///
    fn new() -> Assembler {
        Assembler {
            label_values:   HashMap::new(),
            changed_labels: HashSet::new(),
            bitcode:        vec![],
            bit_pos:        0,
            bit_offset:     0,
            max_passes:     DEFAULT_MAX_PASSES
        }
    }

    ///
    /// Retrieves the Label attached to a label cell
    ///
    fn get_label(&self, label_cell: &CellRef) -> Result<Label, RuntimeError> {
        match &**label_cell {
            // Value should be a label cell generated by AllocLabel
            SafasCell::Any(maybe_label) => {
                if let Some(label) = maybe_label.downcast_ref::<Label>() {
                    Ok(*label)
                } else {
                    Err(RuntimeError::NotALabel(label_cell.clone()))
                }
            },

            // Other types are not a label
            _ => Err(RuntimeError::NotALabel(label_cell.clone()))
        }
    }

    ///
    /// Reads the value of a label contained within a cell
    ///
    fn get_label_value(&mut self, label_cell: &CellRef) -> Result<CellRef, RuntimeError> {
        // Get the label from the cell
        let label = self.get_label(label_cell)?;

        // Read the label value if it's known (labels have the 'nil' value when they're not known yet, and we do further passes)
        if let Some(label_value) = self.label_values.get(&label) {
            // Already know the value of this label
            Ok(label_value.clone())
        } else {
            // Will need more passes to evaluate this label. We use NIL as a placeholder for these labels.
            Ok(NIL.clone())
        }
    }

    ///
    /// Sets the value of a label to a new value
    ///
    fn set_label_value(&mut self, label_cell: &CellRef, value: CellRef) -> Result<CellRef, RuntimeError> {
        // Get the label from the cell
        let label = self.get_label(label_cell)?;

        // If the label already has a value, check if it's the same as the existing value
        if let Some(last_value) = self.label_values.get(&label) {
            if (&**last_value) != (&*value) {
                self.changed_labels.insert(label);
            }
        } else {
            // First time the label has been set, so mark it as changed
            self.changed_labels.insert(label);
        }

        // Update the label value
        self.label_values.insert(label, value.clone());

        // Result is the value
        Ok(value)
    }

    ///
    /// Appends bitcode to this element 
    ///
    fn append_bitcode(&mut self, bitcode: &BitCodeContent) {
        match bitcode {
            BitCodeContent::Empty           => {},
            BitCodeContent::Value(bitcode)  => {
                self.bit_pos = BitCode::position_after(self.bit_pos, bitcode.iter());
                self.bitcode.extend(bitcode.iter().cloned());
            }
        }
    }

    ///
    /// Assembles a single monad using this assembler, returning the monad's value
    ///
    fn assemble(&mut self, monad: &BitCodeMonad) -> Result<CellRef, RuntimeError> {
        // Append the initial bitcode
        self.append_bitcode(&monad.bitcode);

        // Work out the value depending on the content of the monad
        let result = match &monad.value {
            // Simple value
            BitCodeValue::Value(value)                      => Ok(value.clone()),

            // Allocates a new label
            BitCodeValue::AllocLabel                        => Err(RuntimeError::CannotAllocateLabelsDuringAssembly),

            // Looks up a label value (or prepares a second pass if the label has no value yet)
            BitCodeValue::LabelValue(value)                 => self.get_label_value(value),

            // Updates the value of a label, uses the value to map to the next monad
            BitCodeValue::SetLabelValue(label, value)       => self.set_label_value(label, value.clone()),

            // Map based on the current bit position
            BitCodeValue::BitPos                            => {
                let pos     = self.bit_pos;
                let offset  = self.bit_offset;

                if offset < 0 && (-offset as u64) > pos {
                    Err(RuntimeError::BeforeStartOfFile)
                } else {
                    let pos = (pos as i64) + offset;

                    Ok(SafasCell::Number(SafasNumber::BitNumber(64, pos as u128)).into())
                }
            },

            BitCodeValue::SetBitPos(value)                  => {
                // Value must be a number
                let value       = value.number_value().ok_or(RuntimeError::NotANumber(value.clone()))?;
                let value       = value.to_usize() as u64;

                // Update the offset
                let offset      = (value as i64) - (self.bit_pos as i64);

                self.bit_offset = offset;

                Ok(NIL.clone())
            },

            // Value is the result of applying the mapping function to the specified monad, and then trying again with the current monad
            BitCodeValue::FlatMap(monad, mappings)          => {
                // About to call assemble recursively: create a new set of changed labels
                let mut our_labels  = HashSet::new();
                mem::swap(&mut our_labels, &mut self.changed_labels);

                // Loop until the labels in the flat_mapped section acquire stable values
                let initial_bit_pos     = self.bit_pos;
                let initial_code_len    = self.bitcode.len();
                let mut passes          = 0;
                let mut value;
                loop {
                    // The initial value comes from the initial monad
                    value               = self.assemble(monad)?;

                    // Evaluate each mapping in turn
                    for mapping in mappings.iter() {
                        let next_monad  = mapping(value)?;
                        value           = self.assemble(&next_monad)?;
                    }

                    // If there are no changed labels, stop running passes
                    if self.changed_labels.len() == 0 { break; }

                    // Any labels changed here should be merged into our_labels
                    our_labels.extend(self.changed_labels.iter().cloned());

                    // Limit the number of passes we can perform
                    passes += 1;
                    if passes > self.max_passes {
                        return Err(RuntimeError::TooManyPasses(self.max_passes));
                    }

                    // Reset for the next pass
                    self.changed_labels = HashSet::new();
                    self.bit_pos        = initial_bit_pos;
                    self.bitcode.truncate(initial_code_len);
                }

                // Reset with the labels from this level of recursion
                mem::swap(&mut our_labels, &mut self.changed_labels);

                // Final value from the final monad
                Ok(value)
            }
        };

        // Append the following bitcode if there is any
        self.append_bitcode(&monad.following_bitcode);

        // Return the value we generated
        result
    }
}

///
/// Assembles the bitcode generated by a bitcode monad, producing the final bitcode
///
pub fn assemble(monad: &BitCodeMonad) -> Result<(CellRef, Vec<BitCode>), RuntimeError> {
    // Create an assembler, and assemble this monad
    let mut assembler   = Assembler::new();
    let value           = assembler.assemble(monad)?;

    Ok((value, assembler.bitcode))
}

#[cfg(test)]
mod test {
    use crate::interactive::*;
    use crate::bitcode::*;

    #[test]
    fn return_value_from_assembler() {
        let result          = eval("((fun () (d 0u64) 1u64))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        assert!(val.to_string() == "$1u64".to_string());
    }
}
