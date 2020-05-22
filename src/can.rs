//! Controller Area Network

/// CAN Identifier
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Id {
    /// Standard 11bit Identifier (0..=0x7FF)
    Standard(u32),

    /// Extended 29bit Identifier (0..=0x1FFF_FFFF)
    Extended(u32),
}

impl Id {
    /// Returs true when the identifier is valid, false otherwise.
    pub fn valid(self) -> bool {
        match self {
            Id::Standard(id) if id <= 0x7FF => true,
            Id::Extended(id) if id <= 0x1FFF_FFFF => true,
            _ => false,
        }
    }
}

/// A CAN2.0 Frame
pub trait Frame: Sized {
    /// Creates a new frame.
    /// Returns an error when the the identifier is not valid or the data slice is too long.
    fn new(id: Id, data: &[u8]) -> Result<Self, ()>;

    /// Creates a new remote frame (RTR bit set).
    /// Returns an error when the the identifier is  or the data length code (DLC) not valid.
    fn new_remote(id: Id, dlc: usize) -> Result<Self, ()>;

    /// Returns true if this frame is a extended frame.
    fn is_extended(&self) -> bool;

    /// Returns true if this frame is a standard frame.
    fn is_standard(&self) -> bool {
        !self.is_extended()
    }

    /// Returns true if this frame is a remote frame.
    fn is_remote_frame(&self) -> bool;

    /// Returns true if this frame is a data frame.
    fn is_data_frame(&self) -> bool {
        !self.is_remote_frame()
    }

    /// Returns the frame identifier.
    fn id(&self) -> Id;

    /// Returns the data length code (DLC) which is in the range 0..8.
    ///
    /// For data frames the DLC value always matches the length of the data.
    /// Remote frames do not carry any data, yet the DLC can be greater than 0.
    fn dlc(&self) -> usize;

    /// Returns the frame data (0..8 bytes in length).
    fn data(&self) -> &[u8];
}

/// A CAN interface that is able to transmit and receive frames.
pub trait Can {
    /// Associated frame type.
    type Frame: Frame;

    /// Associated error type.
    type Error;

    /// Puts a frame in the transmit buffer.
    ///
    /// If the buffer is full, this function will try to replace a lower priority frame
    /// and return it. This is to avoid the priority inversion problem.
    /// Transmits frames of equal identifier in FIFO fashion.
    fn transmit(&mut self, frame: &Self::Frame) -> nb::Result<Option<Self::Frame>, Self::Error>;

    /// Returns a received frame if available.
    fn receive(&mut self) -> nb::Result<Self::Frame, Self::Error>;
}

/// Filter mask type.
pub enum MaskType {
    /// Each filter of the group has an individual mask.
    Individual,

    /// All filters of a group share a common filter mask.
    Shared,
}

/// Remote frame filter behaviour description.
pub enum RtrFilterBehavior {
    /// The RTR bit is part of the filter and the mask.
    ///
    /// Both `Filter::allow_remote()` and `Filter::remote_only()` are implemented.
    Configurable,

    /// The RTR bit is part of the filter.
    ///
    /// `Filter::remote_only()` is implemented. `Filter::allow_remote()` has no
    /// effect.
    ConfigurableEitherDataOrRemote,

    /// Both data and remote frames with a mathing identifier are accepted.
    ///
    /// `Filter::allow_remote()` nor `Filter::remote_only()` have an effect on the filter configuration.
    RemoteAlwaysAllowed,

    /// Only data remote frames with a mathing identifier are accepted.
    ///
    /// `Filter::allow_remote()` nor `Filter::remote_only()` have an effect on the filter configuration.
    OnlyData,

    /// Only data remote frames with a mathing identifier are accepted.
    ///
    /// `Filter::allow_remote()` nor `Filter::remote_only()` have an effect on the filter configuration.
    OnlyRemote,
}

/// A filter group with its capabilities.
pub trait FilterGroup {
    /// Returns the number of consecutive filter with the same capability.
    fn num_filters(&self) -> usize;

    /// Returs `true` when extended 29bit identifiers are supported (in addition
    /// to the standard 11bit identifiers).
    fn extended(&self) -> bool;

    /// Returns the filter mask type. `None` if no masks is supported.
    fn mask(&self) -> Option<MaskType>;

    /// Returs the filter behavior in regard to remote frames.
    fn rtr(&self) -> RtrFilterBehavior;
}

/// CAN filter interface
pub trait Filter {
    /// Creates a filter that accepts all frames.
    fn accept_all() -> Self;

    /// Creates a filter that accepts frames with the specified identifier.
    fn new(id: Id) -> Self;

    /// Applies a mask to the filter.
    ///
    /// # Example
    ///
    ///    Filter ID:  0b100110111
    ///    Mask:       0b000001111
    ///    
    ///    Receive ID: 0b100110011
    ///                        \----> Not accepted (bit 3 did not match)
    ///    
    ///    Receive ID: 0b000000111 -> accepted
    fn with_mask(&mut self, mask: u32) -> &mut Self;

    /// Makes the filter acccept both data and remote frames.
    ///
    /// Sets the RTR bit in the filter mask.
    /// Only available for filters with `RtrFilterBehavior::Configurable`.
    fn allow_remote(&mut self) -> &mut Self;

    /// Makes the filter acccept remote frames only.
    ///
    /// Sets the RTR bit in the filter and clears it in the mask (if available).
    /// Only available for filters with `RtrFilterBehavior::Configurable` or
    /// `RtrFilterBehavior::ConfigurableEitherDataOrRemote`.
    fn remote_only(&mut self) -> &mut Self;
}

/// A CAN interface that is able to specify receive filters.
pub trait FilteredReceiver: Can {
    /// Associated filter type.
    type Filter: Filter;

    /// Associated filter group type.
    type FilterGroup: FilterGroup;

    /// Associated iterator type for the filter groups.
    type FilterGroups: IntoIterator<Item = Self::FilterGroup>;

    /// Returns the filter's groups.
    fn filter_groups(&self) -> Self::FilterGroups;

    /// Adds and enables a filter.
    fn add_filter(&mut self, filter: &Self::Filter) -> Result<(), Self::Error>;

    /// Clears all filters. No messages can be received anymore.
    fn clear_filters(&mut self);
}
