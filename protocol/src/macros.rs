macro_rules! packets {
    (
        $message:ident, $packet:ident;
        $($id:literal $name:ident $({ $($field:ident : $ty:ty),* $(,)? })?),* $(,)?
    ) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            ::num_enum::TryFromPrimitive,
            ::num_enum::IntoPrimitive,
        )]
        #[repr(u8)]
        pub enum $message {
            $($name = $id),*
        }

        #[derive(Debug, Clone, PartialEq)]
        pub enum $packet {
            $($name $({ $($field: $ty),* })?),*
        }

        impl $packet {
            pub const fn message_type(&self) -> $message {
                match self {
                    $(Self::$name { .. } => $message::$name),*
                }
            }
        }

        impl $crate::message::Message for $packet {
            fn decode(frame: $crate::frame::Frame<'_>) -> $crate::error::ProtocolResult<Self> {
                let kind = $message::try_from(frame.kind)
                    .map_err(|e| $crate::error::ProtocolError::UnknownMessage(e.number))?;
                let mut reader = $crate::codec::Reader::new(frame.payload);

                Ok(match kind {
                    $($message::$name => Self::$name $({
                        $($field: $crate::field::Field::decode(&mut reader)?),*
                    })?),*
                })
            }

            fn encode(&self, out: &mut Vec<u8>) -> $crate::error::ProtocolResult {
                $crate::frame::encode(self.message_type().into(), out, |writer| match self {
                    $(Self::$name $({ $($field),* })? => {
                        $($($crate::field::Field::encode($field, writer)?;)*)?
                        Ok(())
                    }),*
                })
            }
        }
    };
}
