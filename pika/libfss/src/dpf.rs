use crate::prg;
use crate::Group;
use serde::Deserialize;
use serde::Serialize;
use std::mem;
use crate::TupleExt;
use crate::TupleMapToExt;
use crate::prg::PrgOutput;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CorWord{
    seed: prg::PrgSeed,
    bits: (bool, bool),
}

// NEWLY ADDED - Structure for retaining useful eval state params ----- ----- ----- ----- -----
pub struct EvalState{
    seed: Vec<prg::PrgSeed>,
    t_bit: Vec<bool>,
    bit: Vec<bool>,
    tau: Vec<PrgOutput>,
}

impl EvalState {
    fn slice_to_index(&self, index: usize) -> EvalState {
        let seed = self.seed.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let bit = self.bit.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let t_bit = self.t_bit.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());
        let tau = self.tau.get(0..index+1).map_or_else(Vec::new, |s| s.to_vec());

        EvalState { seed, bit, t_bit, tau }
    }
}
// ----- ----- ----- ----- ----- newly added ----- ----- ----- ----- -----

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DPFKey<T> {
    key_idx: bool,
    root_seed: prg::PrgSeed,
    cor_words: Vec<CorWord>,
    word: T,
}

fn gen_cor_word(bit: bool, bits: &mut (bool, bool), seeds: &mut (prg::PrgSeed, prg::PrgSeed)) -> CorWord
{
    let data = seeds.map(|s| s.expand());
    let keep = bit;
    let lose = !keep;

    let mut cw = CorWord {
        seed: data.0.seeds.get(lose) ^ data.1.seeds.get(lose),
        bits: (
            data.0.bits.0 ^ data.1.bits.0 ^ bit ^ true,
            data.0.bits.1 ^ data.1.bits.1 ^ bit,
        ),
    };
    for (b, seed) in seeds.iter_mut() {
        *seed = data.get(b).seeds.get(keep).clone();

        if *bits.get(b) {
            *seed = &*seed ^ &cw.seed;
        }

        let mut newbit = *data.get(b).bits.get(keep);
        if *bits.get(b) {
            newbit ^= cw.bits.get(keep);
        }

        *bits.get_mut(b) = newbit;
    }

    cw
}

// NEWLY ADDED - Helper functions for evalAll method ----- ----- ----- ----- -----

fn u32_to_boolean_vector(num: u32) -> Vec<bool> {
    (0..32).map(|i| ((num >> i) & 1) == 1).rev().collect()
}

fn u16_to_boolean_vector(num: u16) -> Vec<bool> {
    (0..16).map(|i| ((num >> i) & 1) == 1).rev().collect()
}

fn find_first_difference_index(v1: &[bool], v2: &[bool]) -> usize {
    for (i, (b1, b2)) in v1.iter().zip(v2.iter()).enumerate() {
        if b1 != b2 {
            return i as usize;
        }
    }
    v1.len() as usize // this can be moved up for efficiency if vectors are equal - but this shouldn't be the case - two vectors in evalAll will never be exactly same
}
// ----- ----- ----- ----- ----- newly added ----- ----- ----- ----- -----

impl<T> DPFKey<T> where T: prg::FromRng + Clone + Group + std::fmt::Debug
{
    pub fn gen(alpha_bits: &[bool], value:&T) -> (DPFKey<T>, DPFKey<T>) {
        // let root_seeds = (prg::PrgSeed::zero(), prg::PrgSeed::one());
        let root_seeds = (prg::PrgSeed::random(), prg::PrgSeed::random());
        let root_bits = (false, true);

        let mut seeds = root_seeds.clone();
        let mut bits = root_bits;
        let mut cor_words: Vec<CorWord> = Vec::new();
        let mut lastWord:T = T::zero();

        for (i, &bit) in alpha_bits.iter().enumerate() {
            let cw = gen_cor_word(bit, &mut bits, &mut seeds);
            cor_words.push(cw);
            // Generate the last word
            if i==alpha_bits.len()-1{
                let converted = seeds.map(|s| s.convert());
                lastWord.add(&value);
                lastWord.sub(&converted.0.word);
                lastWord.add(&converted.1.word);
                if bits.1 {
                    lastWord.negate();
                }
            }
        }

        (
            DPFKey::<T> {
                key_idx: false,
                root_seed: root_seeds.0,
                cor_words: cor_words.clone(),
                word: lastWord.clone(),
            },
            DPFKey::<T> {
                key_idx: true,
                root_seed: root_seeds.1,
                cor_words: cor_words,
                word:  lastWord,
            },
        )
    }

    pub fn eval(&self, idx: &Vec<bool>) -> T {
        debug_assert!(idx.len() <= self.domain_size());
        debug_assert!(!idx.is_empty());

        let mut seed: prg::PrgSeed = self.root_seed.clone();
        let dir = self.key_idx;
        let mut t_bit:bool = self.key_idx;

        let mut word:T = T::zero();

        for level in 0..idx.len() {
            let bit = idx[level];
            
            // Step 1: compute tau
            // 2 bis, 2 seeds
            // let tau = seed.expand_dir(!dir, dir);
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
        }

        word
    }

// ----- ----- ----- ----- ----- newly added ----- ----- ----- ----- -----
    pub fn evalAll(&self) -> Vec<T> {
        let mut y_vec: Vec<T> = Vec::new();        
        let mut res: T;
        let mut prev_state: EvalState;
        let mut prev_num_bool: Vec<bool>;
        
        let max_value: u16 = u16::MAX;
        let half_value: u16 = max_value / 2;
        
        for i in 0..2 {
            // Start from 0 and 1^k/2 outside of iteration
            let init_16b: u16 = i*(half_value+1);
            let mut init_16b_bool_vec: Vec<bool> = u16_to_boolean_vector(init_16b);

            let iter_start: u16 = i*(half_value+1);
            let iter_end: u16 = iter_start + half_value;

            let (res, state) = Self::stateful_eval_no_prev_state(&self, &init_16b_bool_vec);
            // println!("{} CORR {:?} COMP {:?}", init_16b, res, Self::eval(&self, &init_16b_bool_vec));
            prev_state = state;
            y_vec.push(res);

            prev_num_bool = init_16b_bool_vec;
            
            for num in iter_start..iter_end {
                let mut num_bool_vec: Vec<bool> = u16_to_boolean_vector(num);
                let idx_diff = find_first_difference_index(&prev_num_bool, &num_bool_vec);

                let (res, state) = Self::stateful_eval(&self, &num_bool_vec, &prev_state, idx_diff);
                // println!("CORR {:?} COMP {:?}", Self::eval(&self, &num_bool_vec), res);

                y_vec.push(res);

                prev_state = state;
                prev_num_bool = num_bool_vec;
            }
        }
        y_vec

    }

    pub fn stateful_eval(&self, idx: &Vec<bool>, prev_state: &EvalState, idx_diff: usize) -> (T, EvalState) {
        // INITIALIZE STATE / PARAMETERS
        let mut new_state = prev_state.slice_to_index(idx_diff-1);
        // State of the bit that is exactly previous to the one where the first difference is found
        let start_idx: usize = idx_diff;
        let mut seed: prg::PrgSeed = new_state.seed[new_state.seed.len()-1].clone();
        let mut t_bit:bool = new_state.t_bit[new_state.t_bit.len()-1];

        let mut word:T = T::zero();

        // Start from the index of the first bit difference - otherwise the same as eval but with state update and return
        for level in start_idx..idx.len() {
            let bit = idx[level];
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
            // UPDATE STATE
            new_state.seed.push(seed.clone());
            new_state.t_bit.push(t_bit);
            new_state.bit.push(bit);
            new_state.tau.push(tau);
        }
        (word, new_state)
    }

    pub fn stateful_eval_no_prev_state(&self, idx: &Vec<bool>) -> (T, EvalState) {
        // INITIALIZE STATE / PARAMETERS from 0 (no previous state)
        let mut new_state = EvalState {
            seed: Vec::new(),
            bit: Vec::new(),
            t_bit: Vec::new(),
            tau: Vec::new(),
        };

        let mut seed: prg::PrgSeed = self.root_seed.clone();
        let mut t_bit:bool = self.key_idx;
        
        let mut word:T = T::zero();

        for level in 0..idx.len() {
            let bit = idx[level];
            let tau = seed.expand();
            seed = tau.seeds.get(bit).clone();
            if t_bit{
                seed = &seed ^ &self.cor_words[level].seed;
                let new_bit = *tau.bits.get(bit);
                t_bit = new_bit ^ self.cor_words[level].bits.get(bit);
                
            }else{ //when t_bit is false, update seed and t_bit as orginal expanded tau value
                t_bit = *tau.bits.get(bit);
            }

            if level==idx.len()-1{
                let converted = seed.convert::<T>();
                word.add(&converted.word);
                if t_bit {
                    word.add(&self.word);
                }

                if self.key_idx {
                    word.negate();
                }
            }
            // UPDATE STATE
            new_state.seed.push(seed.clone());
            new_state.t_bit.push(t_bit);
            new_state.bit.push(bit);
            new_state.tau.push(tau);
        }
        (word, new_state)
    }
// ----- ----- ----- ----- ----- newly added ----- ----- ----- ----- -----

    pub fn domain_size(&self) -> usize {
        self.cor_words.len()
    }

    pub fn key_size(&self) -> usize {
        let mut keySize = 0usize;
        keySize += mem::size_of_val(&self.key_idx);
        keySize += mem::size_of_val(&self.root_seed);
        keySize += mem::size_of_val(&*self.cor_words);
        keySize += mem::size_of_val(&self.word);
        // println!("cor_words is {}",mem::size_of_val(&*self.cor_words));
        keySize
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ring::*;
    use crate::Group;

    #[test]
    fn evalCheck() {
        // let mut alpha = vec![true];
        // let mut alpha = vec![true,false];
        let mut alpha = crate::u32_to_bits(3, 7);

        let beta = RingElm::from(117u32);
        let (dpf_key0, dpf_key1) = DPFKey::gen(&alpha, &beta);

        {
            let mut evalResult = RingElm::zero();

            let word0 = dpf_key0.eval(&alpha);
            evalResult.add(&word0);

            let word1 = dpf_key1.eval(&alpha);
            evalResult.add(&word1);

            assert_eq!(evalResult, beta);
        }

        {
            let mut evalResult = RingElm::zero();

            alpha[1] ^= true;

            let word0 = dpf_key0.eval(&alpha);
            evalResult.add(&word0);

            let word1 = dpf_key1.eval(&alpha);
            evalResult.add(&word1);

            assert_eq!(evalResult, RingElm::zero());
        }
    }
}
