use math::round::floor;

pub fn to_chunk_pos<T>((x, y): (T, T)) -> (i32, i32)
where
    T: Copy + Into<f64>,
{
    (floor(x.into() / 50.0, 0) as i32, floor(y.into() / 50.0, 0) as i32)
}

pub fn from_chunk_pos<T>((x, y): (T, T)) -> (i32, i32)
where
    T: Copy + Into<f64>,
{
    (floor(x.into() * 50.0, 0) as i32, floor(y.into() * 50.0, 0) as i32)
}
