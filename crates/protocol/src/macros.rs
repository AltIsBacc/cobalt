macro_rules! define_packets {
    (
        c2s {
            $($c_id:literal $c_name:ident $(<$c_lt:lifetime>)?
                $({ $($c_field:ident : $c_ty:ty),* $(,)? })?),* $(,)?
        } $(,)?
        s2c {
            $($s_id:literal $s_name:ident $(<$s_lt:lifetime>)?
                $({ $($s_field:ident : $s_ty:ty),* $(,)? })?),* $(,)?
        } $(,)?
        both {
            $($b_id:literal $b_name:ident $(<$b_lt:lifetime>)?
                $({ $($b_field:ident : $b_ty:ty),* $(,)? })?),* $(,)?
        } $(,)?
    ) => {
        $(define_packet!($c_id $c_name $(<$c_lt>)? { $($($c_field : $c_ty),*)? });)*
        $(define_packet!($s_id $s_name $(<$s_lt>)? { $($($s_field : $s_ty),*)? });)*
        $(define_packet!($b_id $b_name $(<$b_lt>)? { $($($b_field : $b_ty),*)? });)*

        define_message!(C2s { $($c_id $c_name $(<$c_lt>)?,)* $($b_id $b_name $(<$b_lt>)?,)* });
        define_message!(S2c { $($s_id $s_name $(<$s_lt>)?,)* $($b_id $b_name $(<$b_lt>)?,)* });
    };
}

macro_rules! define_packet {
    ($id:literal $name:ident $(<$lt:lifetime>)? { $($field:ident : $ty:ty),* }) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name $(<$lt>)? {
            $(pub $field: $ty,)*
        }

        impl $(<$lt>)? $crate::packet::Packet for $name $(<$lt>)? {
            const ID: u8 = $id;
        }

        impl<'data> $crate::field::Decode<'data> for $name $(<$lt>)? {
            #[allow(unused_variables)]
            fn decode(
                reader: &mut $crate::codec::Reader<'data>,
            ) -> $crate::error::ProtocolResult<Self> {
                Ok(Self { $($field: $crate::field::Decode::decode(reader)?),* })
            }
        }

        impl $(<$lt>)? $crate::field::Encode for $name $(<$lt>)? {
            #[allow(unused_variables)]
            fn encode(
                &self,
                writer: &mut $crate::codec::Writer<'_>,
            ) -> $crate::error::ProtocolResult {
                $($crate::field::Encode::encode(&self.$field, writer)?;)*
                Ok(())
            }
        }
    };
}

macro_rules! define_message {
    ($name:ident { $($id:literal $variant:ident $(<$lt:lifetime>)?,)* }) => {
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        #[allow(clippy::large_enum_variant)]
        pub enum $name<'data> {
            $($variant($variant $(<$lt>)?),)*
        }

        $(
            impl<'data> From<$variant $(<$lt>)?> for $name<'data> {
                fn from(packet: $variant $(<$lt>)?) -> Self {
                    Self::$variant(packet)
                }
            }
        )*

        impl<'data> $name<'data> {
            pub fn decode(
                frame: $crate::Frame<'data>,
            ) -> $crate::error::ProtocolResult<Option<Self>> {
                let mut reader = $crate::codec::Reader::new(frame.payload);
                Ok(Some(match frame.kind {
                    $($id => Self::$variant($crate::field::Decode::decode(&mut reader)?),)*
                    _ => return Ok(None),
                }))
            }

            pub fn write(&self, out: &mut Vec<u8>) -> $crate::error::ProtocolResult {
                match self {
                    $(Self::$variant(packet) => $crate::frame::write(out, packet),)*
                }
            }
        }
    };
}
