use crate::location::Location;
use crate::message::MsgId;

/* * the structure behind dc_array_t */
#[derive(Debug, Clone)]
pub enum dc_array_t {
    MsgIds(Vec<MsgId>),
    Locations(Vec<Location>),
    Uint(Vec<u32>),
}

impl dc_array_t {
    pub(crate) fn get_id(&self, index: usize) -> u32 {
        match self {
            Self::MsgIds(array) => array[index].to_u32(),
            Self::Locations(array) => array[index].location_id,
            Self::Uint(array) => array[index],
        }
    }

    pub(crate) fn get_location(&self, index: usize) -> &Location {
        if let Self::Locations(array) = self {
            &array[index]
        } else {
            panic!("Not an array of locations")
        }
    }

    /// Returns the number of elements in the array.
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::MsgIds(array) => array.len(),
            Self::Locations(array) => array.len(),
            Self::Uint(array) => array.len(),
        }
    }

    pub(crate) fn search_id(&self, needle: u32) -> Option<usize> {
        (0..self.len()).find(|i| self.get_id(*i) == needle)
    }
}

impl From<Vec<u32>> for dc_array_t {
    fn from(array: Vec<u32>) -> Self {
        dc_array_t::Uint(array)
    }
}

impl From<Vec<MsgId>> for dc_array_t {
    fn from(array: Vec<MsgId>) -> Self {
        dc_array_t::MsgIds(array)
    }
}

impl From<Vec<Location>> for dc_array_t {
    fn from(array: Vec<Location>) -> Self {
        dc_array_t::Locations(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_array() {
        let arr: dc_array_t = Vec::<u32>::new().into();
        assert!(arr.len() == 0);

        let ids: Vec<u32> = (2..1002).collect();
        let arr: dc_array_t = ids.into();

        assert_eq!(arr.len(), 1000);

        for i in 0..1000 {
            assert_eq!(arr.get_id(i), (i + 2) as u32);
        }

        assert_eq!(arr.search_id(10), Some(8));
        assert_eq!(arr.search_id(1), None);
    }

    #[test]
    #[should_panic]
    fn test_dc_array_out_of_bounds() {
        let ids: Vec<u32> = (2..1002).collect();
        let arr: dc_array_t = ids.into();
        arr.get_id(1000);
    }
}
