mod cli_text;

pub use cli_text::{
    finite_float_parse, flag_value_resolve_from_args, float_pair_parse,
    nonnegative_finite_float_parse, required_split, scalar_assignment_parse,
};
