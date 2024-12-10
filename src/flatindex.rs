pub trait FlatIndex<T> {
  fn flat_len(&self) -> usize;
  fn flat_index(&self, i: usize) -> &T;
  fn flat_index_mut(&mut self, i: usize) -> &mut T;
}
impl<T> FlatIndex<T> for T {
  fn flat_len(&self) -> usize {
    1
  }
  fn flat_index(&self, i: usize) -> &T {
    assert!(i == 0);
    self
  }
  fn flat_index_mut(&mut self, i: usize) -> &mut T {
    assert!(i == 0);
    self
  }
}
impl<T> FlatIndex<T> for [T] {
  fn flat_len(&self) -> usize {
    self.len()
  }
  fn flat_index(&self, i: usize) -> &T {
    &self[i]
  }
  fn flat_index_mut(&mut self, i: usize) -> &mut T {
    &mut self[i]
  }
}
impl<T> FlatIndex<T> for Option<T> {
  fn flat_len(&self) -> usize {
    self.is_some() as usize
  }
  fn flat_index(&self, i: usize) -> &T {
    assert!(i == 0);
    self.as_ref().unwrap()
  }
  fn flat_index_mut(&mut self, i: usize) -> &mut T {
    assert!(i == 0);
    self.as_mut().unwrap()
  }
}
impl<T> FlatIndex<T> for Vec<T> {
  fn flat_len(&self) -> usize {
    self.len()
  }
  fn flat_index(&self, i: usize) -> &T {
    &self[i]
  }
  fn flat_index_mut(&mut self, i: usize) -> &mut T {
    &mut self[i]
  }
}
impl<T, const N: usize> FlatIndex<T> for [T; N] {
  fn flat_len(&self) -> usize {
    N
  }
  fn flat_index(&self, i: usize) -> &T {
    &self[i]
  }
  fn flat_index_mut(&mut self, i: usize) -> &mut T {
    &mut self[i]
  }
}

#[macro_export]
macro_rules! flat_combine_len {
    (<$t:ty>) => { 0 };
    (<$t:ty> <$e:expr, $le:ident, $re:ident> $(, $($tt:tt)*)?) => {
        $e.$le() + $crate::flat_combine_len!(<$t> $($($tt)*)?)
    };
    (<$t:ty> $e:expr $(, $($tt:tt)*)?) => {
        $crate::flatindex::FlatIndex::<$t>::flat_len($e) + $crate::flat_combine_len!(<$t> $($($tt)*)?)
    };
    // (<$t:ty> $e:expr $(, $rest_e:expr)* $(,)?) => {
    //     $crate::flatindex::FlatIndex::<$t>::flat_len($e) + $crate::flat_combine_len!(<$t> $($rest_e),*)
    // };
}

#[macro_export]
macro_rules! flat_combine {
    (<$t:ty> $idx:expr $(,)?) => { panic!("Index out of bounds") };
    (<$t:ty> $idx:expr, <$e:expr, $le:ident, $re:ident> $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $e.$le();
        if __idx < __len {
            $e.$re(__idx)
        } else {
            $crate::flat_combine!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
    (<$t:ty> $idx:expr, $e:expr $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $crate::flatindex::FlatIndex::<$t>::flat_len($e);
        if __idx < __len {
            $crate::flatindex::FlatIndex::<$t>::flat_index($e, __idx)
        } else {
            $crate::flat_combine!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
    // (<$t:ty> $idx:expr, $e:expr $(, $rest_e:expr)* $(,)?) => {{
    //     let __idx: usize = $idx;
    //     let __len = $crate::flatindex::FlatIndex::<$t>::flat_len($e);
    //     if __idx < __len {
    //         $crate::flatindex::FlatIndex::<$t>::flat_index($e, __idx)
    //     } else {
    //         $crate::flat_combine!(<$t> __idx - __len $(, $rest_e)*)
    //     }
    // }};
}

#[macro_export]
macro_rules! flat_combine_mut {
    (<$t:ty> $idx:expr $(,)?) => { panic!("Index out of bounds") };
    (<$t:ty> $idx:expr, <$e:expr, $le:ident, $re:ident> $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $e.$le();
        if __idx < __len {
            $e.$re(__idx)
        } else {
            $crate::flat_combine_mut!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
    (<$t:ty> $idx:expr, $e:expr $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $crate::flatindex::FlatIndex::<$t>::flat_len($e);
        if __idx < __len {
            $crate::flatindex::FlatIndex::<$t>::flat_index_mut($e, __idx)
        } else {
            $crate::flat_combine_mut!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
    // (<$t:ty> $idx:expr, $e:expr $(, $rest_e:expr)* $(,)?) => {{
    //     let __idx: usize = $idx;
    //     let __len = $crate::flatindex::FlatIndex::<$t>::flat_len($e);
    //     if __idx < __len {
    //         $crate::flatindex::FlatIndex::<$t>::flat_index_mut($e, __idx)
    //     } else {
    //         $crate::flat_combine_mut!(<$t> __idx - __len $(, $rest_e)*)
    //     }
    // }};
}

#[macro_export]
macro_rules! flat_combine_deref {
    (<$t:ty> $idx:expr $(,)?) => { panic!("Index out of bounds") };
    (<$t:ty> $idx:expr, <$e:expr, $le:ident, $re:ident> $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $e.$le();
        if __idx < __len {
            $e.$re(__idx)
        } else {
            $crate::flat_combine_deref!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
    (<$t:ty> $idx:expr, $e:expr $(, $($tt:tt)*)?) => {{
        let __idx: usize = $idx;
        let __len = $crate::flatindex::FlatIndex::<$t>::flat_len($e);
        if __idx < __len {
            *$crate::flatindex::FlatIndex::<$t>::flat_index($e, __idx)
        } else {
            $crate::flat_combine_deref!(<$t> __idx - __len $(, $($tt)*)?)
        }
    }};
}

#[cfg(test)]
mod tests {
  use std::ops::Index;
  #[test]
  fn combine() {
    let mut a: Vec<usize> = vec![0, 1, 2, 3, 4, 5];
    let mut b: Vec<usize> = vec![6, 7, 8, 9, 10, 11];
    let mut c: Option<usize> = Some(12);
    let mut d: Option<usize> = None;
    let mut e: [usize; 3] = [13, 14, 15];
    assert_eq!(flat_combine_len!(<usize> &a, &b), 12);
    assert_eq!(flat_combine_len!(<usize> &a, &b, &c, &d, &e), 16);
    for i in 0..16 {
      let combined = flat_combine!(<usize> i, &a, <&b, len, index>, &c, &d, &e);
      assert_eq!(combined, &i);
      assert_eq!(
        *flat_combine_mut!(<usize> i, &mut a, <&mut b, len, index>, &mut c, &mut d, &mut e),
        i
      );
      assert_eq!(flat_combine_deref!(<usize> i, &a, &b, &c, &d, &e), i);
    }
  }

  #[test]
  #[should_panic]
  fn out_of_range() {
    let a: Vec<usize> = vec![0, 1, 2, 3, 4, 5];
    let b: Vec<usize> = vec![6, 7, 8, 9, 10, 11];
    let c: Option<usize> = Some(12);
    let d: Option<usize> = None;
    let e: [usize; 3] = [13, 14, 15];
    flat_combine!(<usize> 16, &a, <&b, len, index>, &c, &d, &e);
  }
}
