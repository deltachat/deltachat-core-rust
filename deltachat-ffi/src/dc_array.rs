use crate::location::Location;

/* * the structure behind dc_array_t */
#[derive(Debug, Clone)]
pub enum dc_array_t {
    Locations(Vec<Location>),
    Uint(Vec<u32>),
}

impl dc_array_t {
    pub fn get_id(&self, index: usize) -> u32 {
        match self {
            Self::Locations(array) => array[index].location_id,
            Self::Uint(array) => array[index] as u32,
        }
    }

    pub fn get_location(&self, index: usize) -> &Location {
        if let Self::Locations(array) = self {
            &array[index]
        } else {
            panic!("Not an array of locations")
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        match self {
            Self::Locations(array) => array.len(),
            Self::Uint(array) => array.len(),
        }
    }

    pub fn search_id(&self, needle: u32) -> Option<usize> {
        (0..self.len()).find(|i| self.get_id(*i) == needle)
    }

    pub fn as_ptr(&self) -> *const u32 {
        if let dc_array_t::Uint(v) = self {
            v.as_ptr()
        } else {
            panic!("Attempt to convert array of something other than uints to raw");
        }
    }
}

impl From<Vec<u32>> for dc_array_t {
    fn from(array: Vec<u32>) -> Self {
        dc_array_t::Uint(array)
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
