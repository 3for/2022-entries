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

pub use crate::{
    io::{self, Read, Write},
    FromBytes,
    ToBytes,
    Vec,
};
use crate::{serialize::traits::*, SerializationError};

use std::{borrow::Cow, collections::BTreeMap, marker::PhantomData, rc::Rc, sync::Arc};

impl Valid for bool {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl CanonicalSerialize for bool {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, _compress: Compress) -> Result<(), SerializationError> {
        Ok(self.write_le(&mut writer)?)
    }

    #[inline]
    fn serialized_size(&self, _compress: Compress) -> usize {
        1
    }
}

impl CanonicalDeserialize for bool {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        Ok(bool::read_le(reader)?)
    }
}

impl CanonicalSerialize for String {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, _compress: Compress) -> Result<(), SerializationError> {
        Ok(bincode::serialize_into(&mut writer, self)?)
    }

    #[inline]
    fn serialized_size(&self, _compress: Compress) -> usize {
        self.len() + 8
    }
}

impl Valid for String {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }

    #[inline]
    fn batch_check<'a>(_batch: impl Iterator<Item = &'a Self>) -> Result<(), SerializationError>
    where
        Self: 'a,
    {
        Ok(())
    }
}

impl CanonicalDeserialize for String {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        Ok(bincode::deserialize_from(reader)?)
    }
}

macro_rules! impl_canonical_serialization_uint {
    ($type:ty) => {
        impl CanonicalSerialize for $type {
            #[inline]
            fn serialize_with_mode<W: Write>(
                &self,
                mut writer: W,
                _compress: Compress,
            ) -> Result<(), SerializationError> {
                Ok(writer.write_all(&self.to_le_bytes())?)
            }

            #[inline]
            fn serialized_size(&self, _compress: Compress) -> usize {
                std::mem::size_of::<$type>()
            }
        }
        impl Valid for $type {
            #[inline]
            fn check(&self) -> Result<(), SerializationError> {
                Ok(())
            }

            #[inline]
            fn batch_check<'a>(_batch: impl Iterator<Item = &'a Self>) -> Result<(), SerializationError>
            where
                Self: 'a,
            {
                Ok(())
            }
        }

        impl CanonicalDeserialize for $type {
            #[inline]
            fn deserialize_with_mode<R: Read>(
                mut reader: R,
                _compress: Compress,
                _validate: Validate,
            ) -> Result<Self, SerializationError> {
                let mut bytes = [0u8; std::mem::size_of::<$type>()];
                reader.read_exact(&mut bytes)?;
                Ok(<$type>::from_le_bytes(bytes))
            }
        }
    };
}

impl_canonical_serialization_uint!(u8);
impl_canonical_serialization_uint!(u16);
impl_canonical_serialization_uint!(u32);
impl_canonical_serialization_uint!(u64);
impl_canonical_serialization_uint!(usize);

impl<T: CanonicalSerialize> CanonicalSerialize for Option<T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        self.is_some().serialize_with_mode(&mut writer, compress)?;
        if let Some(item) = self {
            item.serialize_with_mode(&mut writer, compress)?;
        }

        Ok(())
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        8 + self.as_ref().map(|s| s.serialized_size(compress)).unwrap_or(0)
    }
}

impl<T: Valid> Valid for Option<T> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        match self {
            Some(v) => v.check(),
            None => Ok(()),
        }
    }

    #[inline]
    fn batch_check<'a>(batch: impl Iterator<Item = &'a Self> + Send) -> Result<(), SerializationError>
    where
        Self: 'a,
    {
        T::batch_check(batch.map(Option::as_ref).filter(Option::is_some).flatten())
    }
}

impl<T: CanonicalDeserialize> CanonicalDeserialize for Option<T> {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let is_some = bool::deserialize_with_mode(&mut reader, compress, validate)?;
        let data = if is_some { Some(T::deserialize_with_mode(&mut reader, compress, validate)?) } else { None };

        Ok(data)
    }
}

// No-op
impl<T> CanonicalSerialize for std::marker::PhantomData<T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, _writer: W, _compress: Compress) -> Result<(), SerializationError> {
        Ok(())
    }

    #[inline]
    fn serialized_size(&self, _compress: Compress) -> usize {
        0
    }
}

impl<T: Sync> Valid for PhantomData<T> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl<T: Send + Sync> CanonicalDeserialize for std::marker::PhantomData<T> {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        _reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        Ok(std::marker::PhantomData)
    }
}

impl<T: CanonicalSerialize + ToOwned> CanonicalSerialize for Rc<T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        self.as_ref().serialize_with_mode(&mut writer, compress)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        self.as_ref().serialized_size(compress)
    }
}

// impl<T: Valid> Valid for Rc<T> {
//     #[inline]
//     fn check(&self) -> Result<(), SerializationError> {
//         self.as_ref().check()
//     }

//     #[inline]

//     fn batch_check<'a>(batch: impl Iterator<Item = &'a Self>) -> Result<(), SerializationError>
//     where
//         Self: 'a,
//     {
//         T::batch_check(batch.map(|v| v.as_ref()))
//     }
// }

// impl<T: CanonicalDeserialize + ToOwned> CanonicalDeserialize for Rc<T> {
//     #[inline]
//     fn deserialize_with_mode<R: Read>(
//         reader: R,
//         compress: Compress,
//         validate: Validate,
//     ) -> Result<Self, SerializationError> {
//         Ok(Rc::new(T::deserialize_with_mode(reader, compress, validate)?))
//     }
// }

impl<T: CanonicalSerialize + ToOwned> CanonicalSerialize for Arc<T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        self.as_ref().serialize_with_mode(&mut writer, compress)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        self.as_ref().serialized_size(compress)
    }
}

impl<T: Valid + Sync + Send> Valid for Arc<T> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        self.as_ref().check()
    }

    #[inline]

    fn batch_check<'a>(batch: impl Iterator<Item = &'a Self> + Send) -> Result<(), SerializationError>
    where
        Self: 'a,
    {
        T::batch_check(batch.map(|v| v.as_ref()))
    }
}

impl<T: CanonicalDeserialize + ToOwned + Sync + Send> CanonicalDeserialize for Arc<T> {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        Ok(Arc::new(T::deserialize_with_mode(reader, compress, validate)?))
    }
}

impl<'a, T: CanonicalSerialize + ToOwned> CanonicalSerialize for Cow<'a, T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        self.as_ref().serialize_with_mode(&mut writer, compress)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        self.as_ref().serialized_size(compress)
    }
}

// impl<'b, T> Valid for Cow<'b, T>
// where
//     T: ToOwned + Sync + Valid + Send,
//     <T as ToOwned>::Owned: CanonicalDeserialize + Send + Valid,
// {
//     #[inline]
//     fn check(&self) -> Result<(), SerializationError> {
//         <<T as ToOwned>::Owned>::check(self.as_ref().borrow())
//     }

//     #[inline]

//     fn batch_check<'a>(batch: impl Iterator<Item = &'a Self>) -> Result<(), SerializationError>
//     where
//         Self: 'a
//     {
//         <<T as ToOwned>::Owned>::batch_check(batch.map(|v| v.as_ref()))
//     }
// }

// impl<'a, T> CanonicalDeserialize for Cow<'a, T>
// where
//     T: ToOwned + Valid + Valid + Sync + Send,
//     <T as ToOwned>::Owned: CanonicalDeserialize + Valid + Send,
// {
//     #[inline]
//     fn deserialize_with_mode<R: Read>(reader: R, compress: Compress, validate: Validate) -> Result<Self, SerializationError> {
//         Ok(Cow::Owned(<T as ToOwned>::Owned::deserialize_with_mode(reader, compress, validate)?))
//     }
// }

impl<T: CanonicalSerialize> CanonicalSerialize for Vec<T> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        self.as_slice().serialize_with_mode(&mut writer, compress)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        self.as_slice().serialized_size(compress)
    }
}

impl<T: Valid> Valid for Vec<T> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        T::batch_check(self.iter())
    }

    #[inline]
    fn batch_check<'a>(batch: impl Iterator<Item = &'a Self> + Send) -> Result<(), SerializationError>
    where
        Self: 'a,
    {
        T::batch_check(batch.flat_map(|v| v.iter()))
    }
}

impl<T: CanonicalDeserialize> CanonicalDeserialize for Vec<T> {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let len = u64::deserialize_with_mode(&mut reader, compress, validate)?;
        let mut values = Vec::new();
        for _ in 0..len {
            values.push(T::deserialize_with_mode(&mut reader, compress, Validate::No)?);
        }

        if let Validate::Yes = validate {
            T::batch_check(values.iter())?
        }
        Ok(values)
    }
}

impl<T: CanonicalSerialize> CanonicalSerialize for [T] {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        let len = self.len() as u64;
        len.serialize_with_mode(&mut writer, compress)?;
        for item in self.iter() {
            item.serialize_with_mode(&mut writer, compress)?;
        }
        Ok(())
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        8 + self.iter().map(|item| item.serialized_size(compress)).sum::<usize>()
    }
}

impl<'a, T: CanonicalSerialize> CanonicalSerialize for &'a [T] {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        (*self).serialize_with_mode(&mut writer, compress)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        (*self).serialized_size(compress)
    }
}

// Implement Serialization for tuples
macro_rules! impl_tuple {
    ($( $ty: ident : $no: tt, )+) => {
        impl<$($ty, )+> Valid for ($($ty,)+) where
            $($ty: Valid,)+
        {
            #[inline]
            fn check(&self) -> Result<(), SerializationError> {
                $(self.$no.check()?;)*
                Ok(())
            }
        }

        impl<$($ty, )+> CanonicalSerialize for ($($ty,)+) where
            $($ty: CanonicalSerialize,)+
        {
            #[inline]
            fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
                $(self.$no.serialize_with_mode(&mut writer, compress)?;)*
                Ok(())
            }

            #[inline]
            fn serialized_size(&self, compress: Compress) -> usize {
                [$(
                    self.$no.serialized_size(compress),
                )*].iter().sum()
            }
        }

        impl<$($ty, )+> CanonicalDeserialize for ($($ty,)+) where
            $($ty: CanonicalDeserialize,)+
        {
            #[inline]
            fn deserialize_with_mode<R: Read>(mut reader: R, compress: Compress, validate: Validate) -> Result<Self, SerializationError> {
                Ok(($(
                    $ty::deserialize_with_mode(&mut reader, compress, validate)?,
                )+))
            }
        }
    }
}

impl_tuple!(A:0, B:1,);
impl_tuple!(A:0, B:1, C:2,);
impl_tuple!(A:0, B:1, C:2, D:3,);

impl<K, V> CanonicalSerialize for BTreeMap<K, V>
where
    K: CanonicalSerialize,
    V: CanonicalSerialize,
{
    /// Serializes a `BTreeMap` as `len(map) || key 1 || value 1 || ... || key n || value n`.
    fn serialize_with_mode<W: Write>(&self, mut writer: W, compress: Compress) -> Result<(), SerializationError> {
        let len = self.len() as u64;
        len.serialize_with_mode(&mut writer, compress)?;
        for (k, v) in self.iter() {
            k.serialize_with_mode(&mut writer, compress)?;
            v.serialize_with_mode(&mut writer, compress)?;
        }
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        8 + self.iter().map(|(k, v)| k.serialized_size(compress) + v.serialized_size(compress)).sum::<usize>()
    }
}

impl<K: Valid, V: Valid> Valid for BTreeMap<K, V> {
    #[inline]
    fn check(&self) -> Result<(), SerializationError> {
        K::batch_check(self.keys())?;
        V::batch_check(self.values())
    }

    #[inline]
    fn batch_check<'a>(batch: impl Iterator<Item = &'a Self>) -> Result<(), SerializationError>
    where
        Self: 'a,
    {
        let (keys, values): (Vec<_>, Vec<_>) = batch.map(|b| (b.keys(), b.values())).unzip();
        K::batch_check(keys.into_iter().flatten())?;
        V::batch_check(values.into_iter().flatten())
    }
}

impl<K, V> CanonicalDeserialize for BTreeMap<K, V>
where
    K: Ord + CanonicalDeserialize,
    V: CanonicalDeserialize,
{
    /// Deserializes a `BTreeMap` from `len(map) || key 1 || value 1 || ... || key n || value n`.
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let len = u64::deserialize_with_mode(&mut reader, compress, validate)?;
        let mut map = BTreeMap::new();
        for _ in 0..len {
            map.insert(
                K::deserialize_with_mode(&mut reader, compress, validate)?,
                V::deserialize_with_mode(&mut reader, compress, validate)?,
            );
        }
        Ok(map)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_serialize<T: PartialEq + std::fmt::Debug + CanonicalSerialize + CanonicalDeserialize>(data: T) {
        let combinations = [
            (Compress::No, Validate::No),
            (Compress::Yes, Validate::No),
            (Compress::No, Validate::Yes),
            (Compress::Yes, Validate::Yes),
        ];
        for (compress, validate) in combinations {
            let mut serialized = vec![0; data.serialized_size(compress)];
            data.serialize_with_mode(&mut &mut serialized[..], compress).unwrap();
            let de = T::deserialize_with_mode(&mut &serialized[..], compress, validate).unwrap();
            assert_eq!(data, de);
        }
    }

    #[test]
    fn test_bool() {
        test_serialize(true);
        test_serialize(false);
    }

    #[test]
    fn test_uint() {
        test_serialize(192830918usize);
        test_serialize(192830918u64);
        test_serialize(192830918u32);
        test_serialize(22313u16);
        test_serialize(123u8);
    }

    #[test]
    fn test_string() {
        test_serialize("asdf".to_owned());
    }

    #[test]
    fn test_vec() {
        test_serialize(vec![1u64, 2, 3, 4, 5]);
        test_serialize(Vec::<u64>::new());
    }

    #[test]
    fn test_tuple() {
        test_serialize((123u64, 234u32, 999u16));
    }

    #[test]
    fn test_tuple_vec() {
        test_serialize(vec![(123u64, 234u32, 999u16), (123u64, 234u32, 999u16), (123u64, 234u32, 999u16)]);
    }

    #[test]
    fn test_option() {
        test_serialize(Some(3u32));
        test_serialize(None::<u32>);
    }

    #[test]
    fn test_phantomdata() {
        test_serialize(std::marker::PhantomData::<u64>);
    }
}
