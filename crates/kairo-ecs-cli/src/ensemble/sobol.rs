// Copyright (c) 2008, Frances Y. Kuo and Stephen Joe

// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//
//     * Neither the names of the copyright holders nor the names of the
//       University of New South Wales and the University of Waikato
//       and its contributors may be used to endorse or promote products derived
//       from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY
// EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY
// DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
// (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
// ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

const BITS: usize = 32;
const POINT_COUNT: u64 = 1 << BITS;

// Joe–Kuo new-joe-kuo-6.21201, 2010-09-16, dimensions 2 through 16.
// https://web.maths.unsw.edu.au/~fkuo/sobol/new-joe-kuo-6.21201
// SHA-256 68eedd2a4e3b659b9695e7aff0f8ac68718bcf620730fc3d3a8c65df2a067441
const PARAMETERS: [(u32, &[u32]); 15] = [
    (0, &[1]),
    (1, &[1, 3]),
    (1, &[1, 3, 1]),
    (2, &[1, 1, 1]),
    (1, &[1, 1, 3, 3]),
    (4, &[1, 3, 5, 13]),
    (2, &[1, 1, 5, 5, 17]),
    (4, &[1, 1, 5, 5, 5]),
    (7, &[1, 1, 7, 11, 19]),
    (11, &[1, 1, 5, 1, 1]),
    (13, &[1, 1, 1, 3, 11]),
    (14, &[1, 3, 5, 5, 31]),
    (1, &[1, 3, 3, 9, 7, 49]),
    (13, &[1, 1, 1, 15, 21, 21]),
    (16, &[1, 3, 1, 13, 27, 49]),
];

/// Invalid dimension or an index beyond the 32-bit Sobol point space.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SobolError {
    #[error("Sobol dimension {0} is outside 1..={max}", max = SobolSequence::MAX_DIMENSION)]
    Dimension(usize),
    #[error("Sobol index {0} is outside 0..{POINT_COUNT}")]
    Index(u64),
}

/// Unscrambled Joe–Kuo D(6) Sobol sequence, in Gray-code order.
///
/// Points begin with the origin. Coordinates are exact multiples of 2^-32
/// in [0, 1). A balanced block has power-of-two length and an aligned start.
#[derive(Clone, Debug)]
pub struct SobolSequence {
    directions: Vec<[u32; BITS]>,
    index: u64,
}

impl SobolSequence {
    pub const VERSION: &'static str = "joe-kuo-d6-gray-32-v1";
    pub const MAX_DIMENSION: usize = PARAMETERS.len() + 1;
    pub const POINT_COUNT: u64 = POINT_COUNT;

    pub fn new(dimension: usize) -> Result<Self, SobolError> {
        if !(1..=Self::MAX_DIMENSION).contains(&dimension) {
            return Err(SobolError::Dimension(dimension));
        }
        let mut directions = Vec::with_capacity(dimension);
        directions.push(std::array::from_fn(|bit| 1 << (BITS - 1 - bit)));
        for &(coefficients, initial) in &PARAMETERS[..dimension - 1] {
            let degree = initial.len();
            let mut row = [0; BITS];
            for (bit, &value) in initial.iter().enumerate() {
                row[bit] = value << (BITS - 1 - bit);
            }
            for bit in degree..BITS {
                let mut value = row[bit - degree] ^ (row[bit - degree] >> degree);
                for lag in 1..degree {
                    if coefficients & (1 << (degree - 1 - lag)) != 0 {
                        value ^= row[bit - lag];
                    }
                }
                row[bit] = value;
            }
            directions.push(row);
        }
        Ok(Self {
            directions,
            index: 0,
        })
    }

    /// Address a point without advancing the sequence.
    pub fn point_at(&self, index: u64) -> Result<Vec<f64>, SobolError> {
        if index >= POINT_COUNT {
            return Err(SobolError::Index(index));
        }
        let gray = (index ^ (index >> 1)) as u32;
        Ok(self
            .directions
            .iter()
            .map(|directions| {
                let mut word = 0;
                let mut remaining = gray;
                while remaining != 0 {
                    word ^= directions[remaining.trailing_zeros() as usize];
                    remaining &= remaining - 1;
                }
                f64::from(word) / POINT_COUNT as f64
            })
            .collect())
    }

    /// Set the next index to a valid point, leaving the cursor unchanged on error.
    pub fn seek(&mut self, index: u64) -> Result<(), SobolError> {
        if index >= POINT_COUNT {
            return Err(SobolError::Index(index));
        }
        self.index = index;
        Ok(())
    }

    pub fn next_point(&mut self) -> Result<Vec<f64>, SobolError> {
        let point = self.point_at(self.index)?;
        self.index += 1;
        Ok(point)
    }
}

/// Historical multiplier-based sequence for replay of pre-Joe–Kuo studies.
/// This version does not provide standard multidimensional Sobol projections.
#[derive(Clone, Debug)]
pub struct LegacySobolSequence(SobolSequence);

impl LegacySobolSequence {
    pub const VERSION: &'static str = "multiplier-gray-32-v0";

    pub fn new(dimension: usize) -> Self {
        let directions = (0..dimension)
            .map(|d| {
                let value = (d as u32).wrapping_mul(37).wrapping_add(1);
                std::array::from_fn(|bit| value << (BITS - 1 - bit))
            })
            .collect();
        Self(SobolSequence {
            directions,
            index: 0,
        })
    }

    pub fn next_point(&mut self) -> Result<Vec<f64>, SobolError> {
        self.0.next_point()
    }
}
