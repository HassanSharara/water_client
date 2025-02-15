
#[macro_export] macro_rules! check_if_err {
     ($pat:expr,$err:expr) => {
         match $pat {
             Ok(res)=>{res}
             _=> return $err
         }
     };
 }

#[macro_export] macro_rules! check_if_nil {
     ($pat:expr,$err:expr) => {
         match $pat {
             Some(res)=>{res}
             _=> return $err
         }
     };
 }