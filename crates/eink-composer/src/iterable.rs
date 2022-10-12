use windows::core::*;
use windows::Foundation::Collections::*;
use windows::Win32::Foundation::E_BOUNDS;

#[implement(
    windows::Foundation::Collections::IIterator<T>,
)]
struct Iterator<T: RuntimeType + 'static>(std::cell::UnsafeCell<(IIterable<T>, usize)>);

#[allow(non_snake_case)]
impl<T: RuntimeType + 'static> IIterator_Impl<T> for Iterator<T> {
    fn Current(&self) -> Result<T> {
        unsafe {
            let this = self.0.get();
            let owner = (*this).0.as_impl();

            if owner.0.len() > (*this).1 {
                Ok(owner.0[(*this).1].clone())
            } else {
                Err(Error::new(E_BOUNDS, "".into()))
            }
        }
    }

    fn HasCurrent(&self) -> Result<bool> {
        unsafe {
            let this = self.0.get();
            let owner = (*this).0.as_impl();
            Ok(owner.0.len() > (*this).1)
        }
    }

    fn MoveNext(&self) -> Result<bool> {
        unsafe {
            let this = self.0.get();
            let owner = (*this).0.as_impl();
            (*this).1 += 1;
            Ok(owner.0.len() > (*this).1)
        }
    }

    fn GetMany(&self, _items: &mut [T::DefaultType]) -> Result<u32> {
        panic!(); // TODO: arrays still need some work.
    }
}

#[implement(
    windows::Foundation::Collections::IIterable<T>,
)]
pub struct Iterable<T>(pub Vec<T>)
where
    T: RuntimeType + 'static;

#[allow(non_snake_case)]
impl<T: RuntimeType + 'static> IIterable_Impl<T> for Iterable<T> {
    fn First(&self) -> Result<IIterator<T>> {
        Ok(Iterator::<T>((unsafe { self.cast()? }, 0).into()).into())
    }
}
