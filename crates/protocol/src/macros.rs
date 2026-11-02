macro_rules! packets {
    (
        $direction:ident {
            $($id:literal $name:ident $(<$lt:lifetime>)? $({ $($field:ident : $ty:ty),* $(,)? })?),*
            $(,)?
        }
    ) => {
        $(packet!($direction $id $name $(<$lt>)? $({ $($field : $ty),* })?);)*
    };
}

macro_rules! packet {
    ($direction:ident $id:literal $name:ident <$lt:lifetime> { $($field:ident : $ty:ty),* }) => {
        packet!(@item $direction $id $name [<$lt>] [$lt] [$name<$lt>] { $($field : $ty),* });
    };
    ($direction:ident $id:literal $name:ident $({ $($field:ident : $ty:ty),* })?) => {
        packet!(@item $direction $id $name [] ['de] [$name] { $($($field : $ty),*)? });
    };
    (
        @item $direction:ident $id:literal $name:ident
        [$($generics:tt)*] [$de:lifetime] [$($self_ty:tt)*]
        { $($field:ident : $ty:ty),* }
    ) => {
        packet!(@define $name [$($generics)*] { $($field : $ty),* });

        impl $($generics)* $crate::packet::Packet for $($self_ty)* {
            const ID: u8 = $id;
        }

        impl<$de> $crate::field::Decode<$de> for $($self_ty)* {
            #[allow(unused_variables)]
            fn decode(reader: &mut $crate::codec::Reader<$de>) -> $crate::error::ProtocolResult<Self> {
                Ok(Self { $($field: $crate::field::Decode::decode(reader)?),* })
            }
        }

        impl $($generics)* $crate::field::Encode for $($self_ty)* {
            #[allow(unused_variables)]
            fn encode(&self, writer: &mut $crate::codec::Writer<'_>) -> $crate::error::ProtocolResult {
                $($crate::field::Encode::encode(&self.$field, writer)?;)*
                Ok(())
            }
        }

        packet!(@direction $direction [$($generics)*] [$($self_ty)*]);
    };
    (@define $name:ident [$($generics:tt)*] { }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;
    };
    (@define $name:ident [$($generics:tt)*] { $($field:ident : $ty:ty),+ }) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name $($generics)* {
            $(pub $field: $ty),+
        }
    };
    (@direction C2s [$($generics:tt)*] [$($self_ty:tt)*]) => {
        impl $($generics)* $crate::packet::C2s for $($self_ty)* {}
    };
    (@direction S2c [$($generics:tt)*] [$($self_ty:tt)*]) => {
        impl $($generics)* $crate::packet::S2c for $($self_ty)* {}
    };
    (@direction Both [$($generics:tt)*] [$($self_ty:tt)*]) => {
        packet!(@direction C2s [$($generics)*] [$($self_ty)*]);
        packet!(@direction S2c [$($generics)*] [$($self_ty)*]);
    };
}
