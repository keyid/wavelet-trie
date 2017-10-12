use bit_vec_wrap::BitVecWrap;
use std::vec::Vec;

// see paper:
// R. Grossi, G. Ottoviano "The Wavelet Trie: Maintaining an Indexed Sequence of Strings in Compressed Space"
// strings are assumed prefix-free. This can be solved by appending a terminator symbol at the end of the string.

// a node in the wavelet trie
#[derive(Debug)]
pub struct WaveletTrie {
	prefix: BitVecWrap,               // α in the literature
	positions: BitVecWrap,            // β in the literature
	left: Option<Box<WaveletTrie>>,   // left subtrie, if any
	right: Option<Box<WaveletTrie>>   // right subtrie, if any

}

impl WaveletTrie {

	// constructor
	pub fn new() -> Self {
		WaveletTrie {
			left: None,
			right: None,
			prefix: BitVecWrap::new(),
			positions: BitVecWrap::new()
		}
	}

	pub fn from_sequences(sequences: &[BitVecWrap]) -> Self {
		let mut wavelet_trie = WaveletTrie::new();
		wavelet_trie.insert_static(sequences);
		wavelet_trie
	}

	fn insert_static(&mut self, sequences: &[BitVecWrap]) {
		if !sequences.is_empty() {
			// first check if all bitvectors in the sequence are the same
			let first_sequence = &sequences[0];
			let all_equal = sequences.iter().all( |current_sequence| current_sequence == first_sequence);
			if all_equal {
				self.prefix = first_sequence.clone();
				self.positions = BitVecWrap::from_elem(sequences.len(), false)
			} else {
				// create children
				let mut left_child = WaveletTrie::new();
				let mut right_child = WaveletTrie::new();
				// find longest common prefix
				self.prefix = first_sequence.clone();
				for sequence in sequences {
					self.prefix = self.prefix.longest_common_prefix(sequence);
				}
				// split accordingly
				let mut left_sequences: Vec<BitVecWrap> = Vec::new();
				let mut right_sequences: Vec<BitVecWrap> = Vec::new();
				for sequence in sequences {
					let (bit, suffix) = sequence.different_suffix(self.prefix.len());
					self.positions.push(bit);
					if bit {
						right_sequences.push(suffix);
					} else {
						left_sequences.push(suffix);
					}
				}
				// now insert left and right sequences into subtrees
				left_child.insert_static(&left_sequences);
				right_child.insert_static(&right_sequences);
				self.left = Some(Box::new(left_child));
				self.right = Some(Box::new(right_child));
			}
		}
	}

	// append a sequence to the trie at last position
	pub fn append(&mut self, sequence: &BitVecWrap) -> Result<(), &'static str> {
		let index = self.positions.len();
		self.insert(sequence, index)
	}

	// insert a sequence to the trie at some index
	pub fn insert(&mut self, sequence: &BitVecWrap, index: usize) -> Result<(), &'static str> {
		// 1. self.prefix is empty, no children:
		//     self.prefix = sequence
		// 2. self.prefix is empty, children:
		//     2.a. sequence is empty:
		//         ERROR: sequence a prefix of self.prefix
		//     2.b. sequence not empty:
		//         Take the first bit off of sequence; this determines whether the rest of sequence is inserted into the left or right child
		// 3. self.prefix not empty:
		//     3.a. sequence is empty:
		//         ERROR: sequence is prefix of string already in the trie
		//     3.b. self.prefix == sequence:
		//         if no children: OK
		//         if children: ERROR: sequence is prefix of string already in trie
		//     3.c. sequence is prefix of self.prefix:
		//         ERROR
		//     3.d. self.prefix is prefix of sequence:
		//         if children: substract self.prefix from sequence, take the first bit off of sequence; this determines whether the rest of sequence is inserted into the left or right child
		//         if no children: ERROR
		//     else:
		//         (split current node; one child is existing trie and the other a new leaf)
		//         calculate longest common prefix (lcp) of self.prefix and sequence
		//         one new node has as prefix the suffix of self.prefix and the original children
		//         one new node had as prefix the suffix of sequence and no children
		//         self.prefix = lcp; self.left and self.right are the new nodes, determined by the first buit of the calculated suffixes

		if self.prefix.is_empty() {
			// case 1: empty prefix, no children
			if self.left.is_none() {
				self.prefix = sequence.copy();
				self.positions.push(false);
				return Ok(());

			// case 2: empty prefix, children
			} else {
				if sequence.is_empty() {
					return Err("The string being inserded is a prefix of a string in the trie, which is not allowed. (1)");
					//Err("The string being inserded is a prefix of a string in the trie, which is not allowed.")
				} else {
					return self.insert_to_child(sequence, index);
				}
			}

		// case 3: prefix is not empty
		} else {
			if sequence.is_empty() {
				return Err("The string being inserded is a prefix of a string in the trie, which is not allowed. (5)");
			} else if &self.prefix == sequence {
				if self.left.is_none() {
					self.positions.insert(index, false);
					return Ok(());
				} else {
					return Err("The string being inserded is a prefix of a string in the trie, which is not allowed. (2)");
				}
			} else if sequence.is_prefix_of(&self.prefix) {
				return Err("The string being inserded is a prefix of a string in the trie, which is not allowed. (3)");
			} else if self.prefix.is_prefix_of(sequence) {
				if self.left.is_none() {
					return Err("A string in the trie The string being inserded is a prefix of a , which is not allowed. (4)");
				} else {
					return self.insert_to_child(sequence, index);
				}
			} else {
				let lcp = sequence.longest_common_prefix(&self.prefix);
				// bit_self determines wheter original node comes as left or right child in of new node
				// suffix_self becomes prefix in new split node
				let (bit_self, suffix_self) = self.prefix.different_suffix(lcp.len());
				// suffix_seq becomes prefix in new leaf
				let (bit_seq, suffix_seq) = sequence.different_suffix(lcp.len());

				// reconstruct the original node
				let original_left = self.left.take();
				let original_right = self.right.take();
				let original_positions = self.positions.copy();
				let original_node = WaveletTrie {
					left: original_left,
					right: original_right,
					prefix: suffix_self,
					positions: original_positions
				};

				// create the leaf
				let new_leaf = WaveletTrie {
					left: None,
					right: None,
					prefix: suffix_seq,
					positions : BitVecWrap::from_elem(1, false)
				};

				// make this node the new node
				let (new_left, new_right) = match bit_self {
					false => (Some(Box::new(original_node)), Some(Box::new(new_leaf))),
					true => (Some(Box::new(new_leaf)), Some(Box::new(original_node)))
				};
				self.left = new_left;
				self.right = new_right;
				self.prefix = lcp;
				let pos_len = self.positions.len();
				self.positions = BitVecWrap::from_elem(pos_len, bit_self);
				self.positions.insert(index, bit_seq);

				return Ok(())
			}
		}
	}

	fn insert_to_child(&mut self, sequence: &BitVecWrap, index: usize) -> Result<(), &'static str> {
		let (bit, suffix) = sequence.different_suffix(self.prefix.len());
		self.positions.insert(index, bit);

		// simplify this with clojures?
		let result = match bit {
			true => {
				if let Some(ref mut child) = self.right {
					let new_pos = self.positions.rank_one(index);
					child.insert(&suffix, new_pos)
				} else {
					Err("The right child has run away!")
				}
			},
			false => {
				if let Some(ref mut child) = self.left {
					let new_pos = self.positions.rank_zero(index);
					child.insert(&suffix, new_pos)
				} else {
					Err("The left child has run away!")
				}
			}
		};
		return result;
	}

	// count the number of occurrences "sequence" (can be a prefix) up to index − 1.
	// returns None if sequence does not occur
	pub fn rank(&self, sequence: &BitVecWrap, index: usize) -> Option<usize> {
		if sequence.is_empty() || sequence == &self.prefix {
			Some(index)
		} else if sequence.len() < self.prefix.len() {
			// sequence has to be a prefix of "prefix"
			// if so, return "index". If not, the sequence is not in the trie.
			match sequence.is_prefix_of(&self.prefix) {
				true => Some(index),
				false => None
			}
		} else {
			// "prefix" has to be a prefix of sequence
			// if so, substract "prefix" from the beginning of sequence, and recurse!
			match self.prefix.is_prefix_of(sequence) {
				true => {
					let (bit, suffix) = sequence.different_suffix(self.prefix.len());
					match bit {
						true => {
							let new_index = self.positions.rank_one(index);
							match self.right {
								Some(ref trie) => trie.rank(&suffix, new_index),
								None => Some(new_index)
							}
						},
						false => {
							let new_index = self.positions.rank_zero(index);
							match self.left {
								Some(ref trie) => trie.rank(&suffix, new_index),
								None => Some(new_index)
							}
						}
					}
				},
				false => None
			}
		}
	}

	// the number of sequences contained in this trie
	pub fn len(&self) -> usize {
		self.positions.len()
	}
}

mod tests;
