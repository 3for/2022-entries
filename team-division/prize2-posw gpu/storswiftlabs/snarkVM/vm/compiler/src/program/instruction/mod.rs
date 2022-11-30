// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

mod opcode;
pub use opcode::*;

mod operand;
pub use operand::*;

mod operation;
pub use operation::*;

mod bytes;
mod parse;

use crate::{Registers, Stack};
use console::{
    network::{
        prelude::{
            alt,
            bail,
            ensure,
            error,
            fmt,
            map,
            tag,
            Debug,
            Display,
            Error,
            Formatter,
            FromBytes,
            FromStr,
            IoResult,
            Parser,
            ParserResult,
            Read,
            Result,
            Sanitizer,
            ToBytes,
            Write,
        },
        Network,
    },
    program::{Register, RegisterType},
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Instruction<N: Network> {
    /// Compute the absolute value of `first`, checking for overflow, and storing the outcome in `destination`.
    Abs(Abs<N>),
    /// Compute the absolute value of `first`, wrapping around at the boundary of the type, and storing the outcome in `destination`.
    AbsWrapped(AbsWrapped<N>),
    /// Adds `first` with `second`, storing the outcome in `destination`.
    Add(Add<N>),
    /// Adds `first` with `second`, wrapping around at the boundary of the type, and storing the outcome in `destination`.
    AddWrapped(AddWrapped<N>),
    /// Performs a bitwise `and` operation on `first` and `second`, storing the outcome in `destination`.
    And(And<N>),
    /// Calls a closure on the operands.
    Call(Call<N>),
    /// Casts the operands into the declared type.
    Cast(Cast<N>),
    /// Performs a BHP commitment on inputs of 256-bit chunks.
    CommitBHP256(CommitBHP256<N>),
    /// Performs a BHP commitment on inputs of 512-bit chunks.
    CommitBHP512(CommitBHP512<N>),
    /// Performs a BHP commitment on inputs of 768-bit chunks.
    CommitBHP768(CommitBHP768<N>),
    /// Performs a BHP commitment on inputs of 1024-bit chunks.
    CommitBHP1024(CommitBHP1024<N>),
    // /// Performs a Pedersen commitment on up to a 64-bit input.
    // CommitPed64(CommitPed64<N>),
    // /// Performs a Pedersen commitment on up to a 128-bit input.
    // CommitPed128(CommitPed128<N>),
    /// Divides `first` by `second`, storing the outcome in `destination`.
    Div(Div<N>),
    /// Divides `first` by `second`, wrapping around at the boundary of the type, and storing the outcome in `destination`.
    DivWrapped(DivWrapped<N>),
    /// Doubles `first`, storing the outcome in `destination`.
    Double(Double<N>),
    /// Computes whether `first` is greater than `second` as a boolean, storing the outcome in `destination`.
    GreaterThan(GreaterThan<N>),
    /// Computes whether `first` is greater than or equal to `second` as a boolean, storing the outcome in `destination`.
    GreaterThanOrEqual(GreaterThanOrEqual<N>),
    /// Performs a BHP hash on inputs of 256-bit chunks.
    HashBHP256(HashBHP256<N>),
    /// Performs a BHP hash on inputs of 512-bit chunks.
    HashBHP512(HashBHP512<N>),
    /// Performs a BHP hash on inputs of 768-bit chunks.
    HashBHP768(HashBHP768<N>),
    /// Performs a BHP hash on inputs of 1024-bit chunks.
    HashBHP1024(HashBHP1024<N>),
    /// Performs a Pedersen hash on up to a 64-bit input.
    HashPED64(HashPED64<N>),
    /// Performs a Pedersen hash on up to a 128-bit input.
    HashPED128(HashPED128<N>),
    /// Performs a Poseidon hash with an input rate of 2.
    HashPSD2(HashPSD2<N>),
    /// Performs a Poseidon hash with an input rate of 4.
    HashPSD4(HashPSD4<N>),
    /// Performs a Poseidon hash with an input rate of 8.
    HashPSD8(HashPSD8<N>),
    /// Computes the multiplicative inverse of `first`, storing the outcome in `destination`.
    Inv(Inv<N>),
    /// Computes whether `first` equals `second` as a boolean, storing the outcome in `destination`.
    IsEqual(IsEqual<N>),
    /// Computes whether `first` does **not** equals `second` as a boolean, storing the outcome in `destination`.
    IsNotEqual(IsNotEqual<N>),
    /// Computes whether `first` is less than `second` as a boolean, storing the outcome in `destination`.
    LessThan(LessThan<N>),
    /// Computes whether `first` is less than or equal to `second` as a boolean, storing the outcome in `destination`.
    LessThanOrEqual(LessThanOrEqual<N>),
    /// Multiplies `first` with `second`, storing the outcome in `destination`.
    Mul(Mul<N>),
    /// Multiplies `first` with `second`, wrapping around at the boundary of the type, and storing the outcome in `destination`.
    MulWrapped(MulWrapped<N>),
    /// Returns `false` if `first` and `second` are true, storing the outcome in `destination`.
    Nand(Nand<N>),
    /// Negates `first`, storing the outcome in `destination`.
    Neg(Neg<N>),
    /// Returns `true` if neither `first` nor `second` is `true`, storing the outcome in `destination`.
    Nor(Nor<N>),
    /// Flips each bit in the representation of `first`, storing the outcome in `destination`.
    Not(Not<N>),
    /// Performs a bitwise `or` on `first` and `second`, storing the outcome in `destination`.
    Or(Or<N>),
    /// Raises `first` to the power of `second`, storing the outcome in `destination`.
    Pow(Pow<N>),
    /// Raises `first` to the power of `second`, wrapping around at the boundary of the type, storing the outcome in `destination`.
    PowWrapped(PowWrapped<N>),
    /// Shifts `first` left by `second` bits, storing the outcome in `destination`.
    Shl(Shl<N>),
    /// Shifts `first` left by `second` bits, continuing past the boundary of the type, storing the outcome in `destination`.
    ShlWrapped(ShlWrapped<N>),
    /// Shifts `first` right by `second` bits, storing the outcome in `destination`.
    Shr(Shr<N>),
    /// Shifts `first` right by `second` bits, continuing past the boundary of the type, storing the outcome in `destination`.
    ShrWrapped(ShrWrapped<N>),
    /// Squares 'first', storing the outcome in `destination`.
    Square(Square<N>),
    /// Compute the square root of 'first', storing the outcome in `destination`.
    SquareRoot(SquareRoot<N>),
    /// Computes `first - second`, storing the outcome in `destination`.
    Sub(Sub<N>),
    /// Computes `first - second`, wrapping around at the boundary of the type, and storing the outcome in `destination`.
    SubWrapped(SubWrapped<N>),
    /// Selects `first`, if `condition` is true, otherwise selects `second`, storing the result in `destination`.
    Ternary(Ternary<N>),
    /// Performs a bitwise `xor` on `first` and `second`, storing the outcome in `destination`.
    Xor(Xor<N>),
}

/// Creates a match statement that applies the given operation for each instruction.
///
/// ## Example
/// This example will print the opcode and the instruction to the given stream.
/// ```ignore
/// instruction!(self, |instruction| write!(f, "{} {};", self.opcode(), instruction))
/// ```
/// The above example is equivalent to the following logic:
/// ```ignore
///     match self {
///         Self::Add(instruction) => write!(f, "{} {};", self.opcode(), instruction),
///         Self::Sub(instruction) => write!(f, "{} {};", self.opcode(), instruction),
///         Self::Mul(instruction) => write!(f, "{} {};", self.opcode(), instruction),
///         Self::Div(instruction) => write!(f, "{} {};", self.opcode(), instruction),
///     }
/// )
/// ```
#[macro_export]
macro_rules! instruction {
    // A variant **with** curly braces:
    // i.e. `instruction!(self, |instruction| { operation(instruction) })`.
    ($object:expr, |$input:ident| $operation:block) => {{ $crate::instruction!(instruction, $object, |$input| $operation) }};
    // A variant **without** curly braces:
    // i.e. `instruction!(self, |instruction| operation(instruction))`.
    ($object:expr, |$input:ident| $operation:expr) => {{ $crate::instruction!(instruction, $object, |$input| { $operation }) }};
    // A variant **with** curly braces:
    // i.e. `instruction!(custom_macro, self, |instruction| { operation(instruction) })`.
    ($macro_:ident, $object:expr, |$input:ident| $operation:block) => {
        $macro_!{$object, |$input| $operation, {
            Abs,
            AbsWrapped,
            Add,
            AddWrapped,
            And,
            Call,
            Cast,
            CommitBHP256,
            CommitBHP512,
            CommitBHP768,
            CommitBHP1024,
            // CommitPed64,
            // CommitPed128,
            Div,
            DivWrapped,
            Double,
            GreaterThan,
            GreaterThanOrEqual,
            HashBHP256,
            HashBHP512,
            HashBHP768,
            HashBHP1024,
            HashPED64,
            HashPED128,
            HashPSD2,
            HashPSD4,
            HashPSD8,
            Inv,
            IsEqual,
            IsNotEqual,
            LessThan,
            LessThanOrEqual,
            Mul,
            MulWrapped,
            Nand,
            Neg,
            Nor,
            Not,
            Or,
            Pow,
            PowWrapped,
            Shl,
            ShlWrapped,
            Shr,
            ShrWrapped,
            Square,
            SquareRoot,
            Sub,
            SubWrapped,
            Ternary,
            Xor,
        }}
    };
    // A variant **without** curly braces:
    // i.e. `instruction!(custom_macro, self, |instruction| operation(instruction))`.
    ($macro_:ident, $object:expr, |$input:ident| $operation:expr) => {{ $crate::instruction!($macro_, $object, |$input| { $operation }) }};
    // A variant invoking a macro internally:
    // i.e. `instruction!(instruction_to_bytes_le!(self, writer))`.
    ($macro_:ident!($object:expr, $input:ident)) => {{ $crate::instruction!($macro_, $object, |$input| {}) }};

    ////////////////////
    // Private Macros //
    ////////////////////

    // A static variant **with** curly braces:
    // i.e. `instruction!(self, |InstructionMember| { InstructionMember::opcode() })`.
    ($object:expr, |InstructionMember| $operation:block, { $( $variant:ident, )+ }) => {{
        // Build the match cases.
        match $object {
            $( Self::$variant(..) => {{
                // Set the variant to be called `InstructionMember`.
                type InstructionMember<N> = $variant<N>;
                // Perform the operation.
                $operation
            }} ),+
        }
    }};
    // A static variant **without** curly braces:
    // i.e. `instruction!(self, |InstructionMember| InstructionMember::opcode())`.
    ($object:expr, |InstructionMember| $operation:expr, { $( $variant:ident, )+ }) => {{
        $crate::instruction!($object, |InstructionMember| { $operation }, { $( $variant, )+ })
    }};
    // A non-static variant **with** curly braces:
    // i.e. `instruction!(self, |instruction| { operation(instruction) })`.
    ($object:expr, |$instruction:ident| $operation:block, { $( $variant:ident, )+ }) => {{
        // Build the match cases.
        match $object { $( Self::$variant($instruction) => { $operation } ),+ }
    }};
    // A non-static variant **without** curly braces:
    // i.e. `instruction!(self, |instruction| operation(instruction))`.
    ($object:expr, |$instruction:ident| $operation:expr, { $( $variant:ident, )+ }) => {{
        $crate::instruction!($object, |$instruction| { $operation }, { $( $variant, )+ })
    }};
}

/// Derives `From<Operation>` for the instruction.
///
/// ## Example
/// ```ignore
/// derive_from_operation!(Instruction, |None| {}, { Add, Sub, Mul, Div })
/// ```
macro_rules! derive_from_operation {
    ($_object:expr, |$_reader:ident| $_operation:block, { $( $variant:ident, )+ }) => {
        $(impl<N: Network> From<$variant<N>> for Instruction<N> {
            #[inline]
            fn from(operation: $variant<N>) -> Self {
                Self::$variant(operation)
            }
        })+
    }
}
instruction!(derive_from_operation, Instruction, |None| {});

/// Returns a slice of all instruction opcodes.
///
/// ## Example
/// ```ignore
/// opcodes!(Instruction, |None| {}, { Add, Sub, Mul, Div })
/// ```
macro_rules! opcodes {
    ($_object:expr, |$_reader:ident| $_operation:block, { $( $variant:ident, )+ }) => { [$( $variant::<N>::opcode() ),+] }
}

impl<N: Network> Instruction<N> {
    /// The list of all instruction opcodes.
    pub const OPCODES: &'static [Opcode] = &instruction!(opcodes, Instruction, |None| {});

    /// Returns the opcode of the instruction.
    #[inline]
    pub const fn opcode(&self) -> Opcode {
        instruction!(self, |InstructionMember| InstructionMember::<N>::opcode())
    }

    /// Returns the operands of the instruction.
    #[inline]
    pub fn operands(&self) -> &[Operand<N>] {
        instruction!(self, |instruction| instruction.operands())
    }

    /// Returns the destination register of the instruction.
    #[inline]
    pub fn destinations(&self) -> Vec<Register<N>> {
        instruction!(self, |instruction| instruction.destinations())
    }

    /// Evaluates the instruction.
    #[inline]
    pub fn evaluate<A: circuit::Aleo<Network = N>>(
        &self,
        stack: &Stack<N>,
        registers: &mut Registers<N, A>,
    ) -> Result<()> {
        instruction!(self, |instruction| instruction.evaluate::<A>(stack, registers))
    }

    /// Executes the instruction.
    #[inline]
    pub fn execute<A: circuit::Aleo<Network = N>>(
        &self,
        stack: &Stack<N>,
        registers: &mut Registers<N, A>,
    ) -> Result<()> {
        instruction!(self, |instruction| instruction.execute::<A>(stack, registers))
    }

    /// Returns the output type from the given input types.
    #[inline]
    pub fn output_types(&self, stack: &Stack<N>, input_types: &[RegisterType<N>]) -> Result<Vec<RegisterType<N>>> {
        instruction!(self, |instruction| instruction.output_types(stack, input_types))
    }
}

impl<N: Network> Debug for Instruction<N> {
    /// Prints the instruction as a string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<N: Network> Display for Instruction<N> {
    /// Prints the instruction as a string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        instruction!(self, |instruction| write!(f, "{};", instruction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::network::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_opcodes() {
        // Sanity check the number of instructions is unchanged.
        assert_eq!(
            49,
            Instruction::<CurrentNetwork>::OPCODES.len(),
            "Update me if the number of instructions changes."
        );
    }
}
