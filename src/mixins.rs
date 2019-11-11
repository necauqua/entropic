
macro_rules! impl_output_mixin {
    (
        mixin $trait:ident: $name:ident<$w:ident$(+ $extra:tt)*> {
            reset => $reset:ident;
            fn $func:ident($self:ident) -> $res:ty;
        }
    ) => {
        impl_output_mixin! {
            mixin $trait: $name<$w$(+ $extra)*> {
                reset => $reset;
                fn $func($self) -> $res {
                    $name { output: $self }
                }
            }
        }
    };
    (
        mixin $trait:ident: $name:ident<$w:ident$(+ $extra:tt)*> {
            apply => $apply:ident;
            reset => $reset:ident;
            fn $func:ident($self:ident) -> $res:ty;
        }
    ) => {
        impl_output_mixin! {
            mixin $trait: $name<$w$(+ $extra)*> {
                reset => $reset;
                fn $func($self) -> $res {
                    let mut wrapper = $name { output: $self };
                    wrapper.$apply()?;
                    wrapper.flush()?;
                    Ok(wrapper)
                }
            }
        }
    };
    (
        mixin $trait:ident: $name:ident<$w:ident$(+ $extra:tt)*> {
            reset => $reset:ident;
            fn $func:ident($self:ident) -> $res:ty {
                $($code:tt)*
            }
        }
    ) => {

        // create a trait for mixin constructor method
        pub trait $trait<$w: std::io::Write $(+ $extra)*> {

            fn $func(self) -> $res;
        }

        // implement it
        impl<$w: std::io::Write $(+ $extra)*> $trait<$w> for $w {
            fn $func($self) -> $res {
                $($code)*
            }
        }

        // implement Drop for automatically resetting the terminal state on drop
        impl<W: std::io::Write $(+ $extra)*> std::ops::Drop for $name<W> {
            fn drop(&mut self) {
                self.$reset().unwrap();
                self.flush().unwrap();
            }
        }

        // re-implement Write because other mixins depended on it
        impl<W: std::io::Write $(+ $extra)*> Write for $name<W> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.output.write(buf)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.output.flush()
            }
        }

        // implement Deref and DerefMut to pretend like underlying type
        impl<W: std::io::Write $(+ $extra)*> std::ops::Deref for $name<W> {
            type Target = W;

            fn deref(&self) -> &W {
                &self.output
            }
        }

        impl<W: std::io::Write $(+ $extra)*> std::ops::DerefMut for $name<W> {
            fn deref_mut(&mut self) -> &mut W {
                &mut self.output
            }
        }
    };
}
