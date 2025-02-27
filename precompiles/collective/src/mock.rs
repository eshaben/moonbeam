// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! Test utilities
use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU128, ConstU64, Everything, GenesisBuild, MapSuccess, OnFinalize, OnInitialize},
	PalletId,
};
use pallet_evm::{
	AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileSet, SubstrateBlockHashMapping,
};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, Replace},
	Permill,
};

pub type AccountId = Account;
pub type Balance = u128;
pub type BlockNumber = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

pub const PRECOMPILE_ADDRESS: u64 = 1;

#[derive(
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Clone,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	Serialize,
	Deserialize,
	derive_more::Display,
	TypeInfo,
)]
pub enum Account {
	Alice,
	Bob,
	Charlie,
	Bogus,
	Precompile,
}

impl Default for Account {
	fn default() -> Self {
		Self::Bogus
	}
}

impl AddressMapping<Account> for Account {
	fn into_account_id(h160_account: H160) -> Account {
		match h160_account {
			a if a == H160::repeat_byte(0xAA) => Self::Alice,
			a if a == H160::repeat_byte(0xBB) => Self::Bob,
			a if a == H160::repeat_byte(0xCC) => Self::Charlie,
			a if a == H160::from_low_u64_be(PRECOMPILE_ADDRESS) => Self::Precompile,
			_ => Self::Bogus,
		}
	}
}

impl From<H160> for Account {
	fn from(x: H160) -> Account {
		Account::into_account_id(x)
	}
}

impl From<Account> for H160 {
	fn from(value: Account) -> H160 {
		match value {
			Account::Alice => H160::repeat_byte(0xAA),
			Account::Bob => H160::repeat_byte(0xBB),
			Account::Charlie => H160::repeat_byte(0xCC),
			Account::Precompile => H160::from_low_u64_be(PRECOMPILE_ADDRESS),
			Account::Bogus => Default::default(),
		}
	}
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Evm: pallet_evm::{Pallet, Config, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Treasury: pallet_treasury::{Pallet, Storage, Config, Event<T>, Call},
		CouncilCollective:
			pallet_collective::<Instance1>::{Pallet, Call, Storage, Event<T>, Origin<T>, Config<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}
impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = Account;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
}

impl pallet_balances::Config for Runtime {
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub BlockGasLimit: U256 = U256::max_value();
	pub const PrecompilesValue: Precompiles<Runtime> = Precompiles(PhantomData);
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = ();
	type GasWeightMapping = ();
	type CallOrigin = EnsureAddressRoot<Account>;
	type WithdrawOrigin = EnsureAddressNever<Account>;
	type AddressMapping = Account;
	type Currency = Balances;
	type Event = Event;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type PrecompilesType = Precompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ();
	type OnChargeTransaction = ();
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = SubstrateBlockHashMapping<Self>;
	type FindAuthor = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 10;
	pub const VotingPeriod: BlockNumber = 10;
	pub const VoteLockingPeriod: BlockNumber = 10;
	pub const FastTrackVotingPeriod: BlockNumber = 5;
	pub const EnactmentPeriod: BlockNumber = 10;
	pub const CooloffPeriod: BlockNumber = 10;
	pub const MinimumDeposit: Balance = 10;
	pub const MaxVotes: u32 = 10;
	pub const MaxProposals: u32 = 10;
	pub const PreimageByteDeposit: Balance = 10;
	pub const InstantAllowed: bool = false;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const TreasuryId: PalletId = PalletId(*b"pc/trsry");
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryId;
	type Currency = Balances;
	type ApproveOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	type RejectOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	type Event = Event;
	// If spending proposal rejected, transfer proposer bond to treasury
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ConstU128<1>;
	type SpendPeriod = ConstU64<1>;
	type Burn = ();
	type BurnDestination = ();
	type MaxApprovals = ConstU32<100>;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type SpendFunds = ();
	type ProposalBondMaximum = ();
	type SpendOrigin = MapSuccess<
		pallet_collective::EnsureProportionMoreThan<AccountId, pallet_collective::Instance1, 1, 2>,
		Replace<ConstU128<1000>>,
	>;
}

impl pallet_collective::Config<pallet_collective::Instance1> for Runtime {
	type Origin = Origin;
	type Event = Event;
	type Proposal = Call;
	/// The maximum amount of time (in blocks) for council members to vote on motions.
	/// Motions may end in fewer blocks if enough votes are cast to determine the result.
	type MotionDuration = ConstU64<2>;
	/// The maximum number of Proposlas that can be open in the council at once.
	type MaxProposals = ConstU32<100>;
	/// The maximum number of council members.
	type MaxMembers = ConstU32<100>;
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

#[derive(Default)]
pub struct Precompiles<R>(PhantomData<R>);

impl<R> PrecompileSet for Precompiles<R>
where
	CollectivePrecompile<R, pallet_collective::Instance1>: Precompile,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
		match handle.code_address() {
			a if a == hash(PRECOMPILE_ADDRESS) => {
				Some(CollectivePrecompile::<R, pallet_collective::Instance1>::execute(handle))
			}
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160) -> bool {
		address == hash(PRECOMPILE_ADDRESS)
	}
}

fn hash(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}

/// Build test externalities, prepopulated with data for testing democracy precompiles
pub(crate) struct ExtBuilder {
	/// Endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
	/// Collective members
	collective: Vec<Account>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			collective: vec![Account::Bob, Account::Charlie],
		}
	}
}

impl ExtBuilder {
	/// Fund some accounts before starting the test
	#[allow(unused)]
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	/// Set members of the collective
	#[allow(unused)]
	pub(crate) fn with_collective(mut self, collective: Vec<AccountId>) -> Self {
		self.collective = collective;
		self
	}

	/// Build the test externalities for use in tests
	#[allow(unused)]
	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self.balances.clone(),
		}
		.assimilate_storage(&mut t)
		.expect("Pallet balances storage can be assimilated");

		pallet_collective::GenesisConfig::<Runtime, pallet_collective::Instance1> {
			members: self.collective.clone(),
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.expect("Pallet collective storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::set_block_number(1);
		});
		ext
	}
}

#[allow(unused)]
pub(crate) fn roll_to(n: u64) {
	// We skip timestamp's on_finalize because it requires that the timestamp inherent be set
	// We may be able to simulate this by poking its storage directly, but I don't see any value
	// added from doing that.
	while System::block_number() < n {
		CouncilCollective::on_finalize(System::block_number());
		Treasury::on_finalize(System::block_number());
		// Times tamp::on_finalize(System::block_number());
		Evm::on_finalize(System::block_number());
		Balances::on_finalize(System::block_number());
		System::on_finalize(System::block_number());

		System::set_block_number(System::block_number() + 1);

		System::on_initialize(System::block_number());
		Balances::on_initialize(System::block_number());
		Evm::on_initialize(System::block_number());
		Timestamp::on_initialize(System::block_number());
		Treasury::on_initialize(System::block_number());
		CouncilCollective::on_initialize(System::block_number());
	}
}

pub(crate) fn events() -> Vec<Event> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.collect::<Vec<_>>()
}

#[macro_export]
macro_rules! assert_tail_eq {
	($tail:expr, $arr:expr) => {
		if $tail.len() != 0 {
			// 0-length always passes

			if $tail.len() > $arr.len() {
				similar_asserts::assert_eq!($tail, $arr); // will fail
			}

			let len_diff = $arr.len() - $tail.len();
			similar_asserts::assert_eq!($tail, $arr[len_diff..]);
		}
	};
}

/// Panics if an event is not found in the system log of events
#[macro_export]
macro_rules! assert_event_emitted {
	($event:expr) => {
		match &$event {
			e => {
				assert!(
					crate::mock::events().iter().find(|x| *x == e).is_some(),
					"Event {:?} was not found in events: \n {:#?}",
					e,
					crate::mock::events()
				);
			}
		}
	};
}

#[test]
fn test_account_id_mapping_works() {
	// Bidirectional conversions for normal accounts
	assert_eq!(
		Account::Alice,
		Account::into_account_id(Account::Alice.into())
	);
	assert_eq!(Account::Bob, Account::into_account_id(Account::Bob.into()));
	assert_eq!(
		Account::Charlie,
		Account::into_account_id(Account::Charlie.into())
	);

	// Bidirectional conversion between bogus and default H160
	assert_eq!(Account::Bogus, Account::into_account_id(H160::default()));
	assert_eq!(H160::default(), Account::Bogus.into());

	// All other H160s map to bogus
	assert_eq!(Account::Bogus, Account::into_account_id(H160::zero()));
	assert_eq!(
		Account::Bogus,
		Account::into_account_id(H160::repeat_byte(0x12))
	);
	assert_eq!(
		Account::Bogus,
		Account::into_account_id(H160::repeat_byte(0xFF))
	);
}
