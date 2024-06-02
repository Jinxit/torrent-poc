use nom::IResult;

pub trait SansIo: Sized {
    fn decode(i: &[u8]) -> IResult<&[u8], Self>;
    fn encode(&self) -> Vec<u8>;
}
