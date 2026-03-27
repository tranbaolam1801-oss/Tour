#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, contracterror,
    symbol_short, vec,
    Address, Env, String, Symbol, Vec,
};

// ─── Storage Keys ─────────────────────────────────────────────────────────────

const ADMIN: Symbol      = symbol_short!("ADMIN");
const ID_COUNT: Symbol   = symbol_short!("IDCOUNT");

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    TravelID(Address),              // address → TravelerProfile
    ProviderID(Address),            // address → ProviderProfile
    CheckIn(Address, u64),          // (traveler, checkin_id) → CheckInRecord
    TravelerCheckIns(Address),      // traveler → Vec<u64>
    PlaceCheckIns(String),          // place_id → Vec<Address>
    Review(Address, Address),       // (reviewer, provider) → ReviewRecord
    ProviderReviews(Address),       // provider → Vec<Address>
    Badge(Address, u32),            // (owner, badge_id) → BadgeRecord
    OwnerBadges(Address),           // owner → Vec<u32>
    PaymentRep(Address),            // address → PaymentReputation
    Place(String),                  // place_id → PlaceInfo
}

// ─── Data Structures ──────────────────────────────────────────────────────────

/// Traveler's on-chain passport
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TravelerProfile {
    pub owner: Address,
    pub display_name: String,
    pub country_code: String,
    pub total_checkins: u32,
    pub total_reviews: u32,
    pub countries_visited: u32,
    pub trust_score: u32,           // 0–1000 composite
    pub registered_at: u64,
}

/// Service provider identity (tour, hotel, guide, restaurant…)
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderProfile {
    pub owner: Address,
    pub business_name: String,
    pub category: String,           // "hotel" | "tour" | "guide" | "restaurant"
    pub country_code: String,
    pub license_number: String,
    pub avg_rating: u32,            // stored as rating*100 (avoid floats)
    pub total_ratings: u32,
    pub is_verified: bool,
    pub registered_at: u64,
}

/// A provider-confirmed visit to a destination
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CheckInRecord {
    pub id: u64,
    pub traveler: Address,
    pub place_id: String,
    pub place_name: String,
    pub country_code: String,
    pub confirmed_by: Address,      // verified provider who stamped the visit
    pub visited_at: u64,
    pub note: String,
}

/// Verified review (must link to a real check-in — anti-fake protection)
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewRecord {
    pub reviewer: Address,
    pub provider: Address,
    pub checkin_id: u64,
    pub rating: u32,                // 1–5
    pub comment: String,
    pub reviewed_at: u64,
    pub is_flagged: bool,
}

/// Soulbound achievement badge
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeRecord {
    pub id: u32,
    pub owner: Address,
    pub badge_type: BadgeType,
    pub title: String,
    pub issued_at: u64,
    pub is_revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum BadgeType {
    CountriesVisited(u32),
    CheckInMilestone(u32),
    TrustedReviewer,
    VerifiedProvider,
    PaymentChampion,
    ExplorerElite,
}

/// Payment reputation record
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PaymentReputation {
    pub owner: Address,
    pub total_deposits: u32,
    pub successful_refunds: u32,
    pub disputes: u32,
    pub dispute_wins: u32,
    pub score: u32,                 // 0–100
    pub last_updated: u64,
}

/// Registered destination / place of interest
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceInfo {
    pub id: String,
    pub name: String,
    pub country_code: String,
    pub category: String,           // "city" | "landmark" | "beach" | "mountain"
    pub description: String,
    pub total_visitors: u32,
    pub avg_rating: u32,
    pub registered_by: Address,
    pub registered_at: u64,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TravelError {
    AlreadyInitialized   = 1,
    NotInitialized       = 2,
    Unauthorized         = 3,
    ProfileNotFound      = 4,
    ProfileAlreadyExists = 5,
    CheckInNotFound      = 6,
    ReviewNotFound       = 7,
    ReviewAlreadyExists  = 8,
    BadgeNotFound        = 9,
    PlaceNotFound        = 10,
    InvalidRating        = 11,
    InvalidInput         = 12,
    CheckInNotConfirmed  = 13,
    BadgeRevoked         = 14,
    ProviderNotVerified  = 15,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn next_badge_id(env: &Env, owner: &Address) -> u32 {
    let key = DataKey::OwnerBadges(owner.clone());
    let badges: Vec<u32> = env.storage().persistent().get(&key).unwrap_or(vec![env]);
    badges.len() as u32 + 1
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct TravelIDContract;

#[contractimpl]
impl TravelIDContract {

    // ── Initialization ────────────────────────────────────────────────────

    pub fn initialize(env: Env, admin: Address) -> Result<(), TravelError> {
        if env.storage().instance().has(&ADMIN) {
            return Err(TravelError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&ADMIN, &admin);
        env.storage().instance().set(&ID_COUNT, &0u64);
        env.storage().instance().extend_ttl(100_000, 200_000);
        Ok(())
    }

    // ── Traveler ──────────────────────────────────────────────────────────

    pub fn register_traveler(
        env: Env,
        owner: Address,
        display_name: String,
        country_code: String,
    ) -> Result<(), TravelError> {
        owner.require_auth();
        Self::assert_initialized(&env)?;
        if env.storage().persistent().has(&DataKey::TravelID(owner.clone())) {
            return Err(TravelError::ProfileAlreadyExists);
        }
        let profile = TravelerProfile {
            owner: owner.clone(),
            display_name,
            country_code,
            total_checkins: 0,
            total_reviews: 0,
            countries_visited: 0,
            trust_score: 500,
            registered_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::TravelID(owner.clone()), &profile);
        env.storage().persistent().extend_ttl(&DataKey::TravelID(owner.clone()), 100_000, 200_000);

        // Init payment reputation alongside profile
        let rep = PaymentReputation {
            owner: owner.clone(),
            total_deposits: 0, successful_refunds: 0,
            disputes: 0, dispute_wins: 0,
            score: 100, last_updated: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::PaymentRep(owner), &rep);
        Ok(())
    }

    pub fn get_traveler(env: Env, owner: Address) -> Result<TravelerProfile, TravelError> {
        env.storage().persistent()
            .get(&DataKey::TravelID(owner))
            .ok_or(TravelError::ProfileNotFound)
    }

    // ── Provider ──────────────────────────────────────────────────────────

    pub fn register_provider(
        env: Env,
        owner: Address,
        business_name: String,
        category: String,
        country_code: String,
        license_number: String,
    ) -> Result<(), TravelError> {
        owner.require_auth();
        Self::assert_initialized(&env)?;
        if env.storage().persistent().has(&DataKey::ProviderID(owner.clone())) {
            return Err(TravelError::ProfileAlreadyExists);
        }
        let profile = ProviderProfile {
            owner: owner.clone(), business_name, category,
            country_code, license_number,
            avg_rating: 0, total_ratings: 0, is_verified: false,
            registered_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::ProviderID(owner.clone()), &profile);
        env.storage().persistent().extend_ttl(&DataKey::ProviderID(owner.clone()), 100_000, 200_000);
        Ok(())
    }

    pub fn get_provider(env: Env, owner: Address) -> Result<ProviderProfile, TravelError> {
        env.storage().persistent()
            .get(&DataKey::ProviderID(owner))
            .ok_or(TravelError::ProfileNotFound)
    }

    /// Admin verifies a provider's license → auto-mints VerifiedProvider badge
    pub fn verify_provider(
        env: Env,
        admin: Address,
        provider: Address,
    ) -> Result<(), TravelError> {
        Self::assert_admin(&env, &admin)?;
        admin.require_auth();
        let mut profile: ProviderProfile = env.storage().persistent()
            .get(&DataKey::ProviderID(provider.clone()))
            .ok_or(TravelError::ProfileNotFound)?;
        profile.is_verified = true;
        env.storage().persistent().set(&DataKey::ProviderID(provider.clone()), &profile);
        Self::_mint_badge(
            &env, provider,
            BadgeType::VerifiedProvider,
            String::from_str(&env, "Nhà cung cấp đã xác minh"),
        );
        Ok(())
    }

    // ── Place Registry ────────────────────────────────────────────────────

    pub fn register_place(
        env: Env,
        registrar: Address,
        place_id: String,
        name: String,
        country_code: String,
        category: String,
        description: String,
    ) -> Result<(), TravelError> {
        registrar.require_auth();
        Self::assert_initialized(&env)?;
        if env.storage().persistent().has(&DataKey::Place(place_id.clone())) {
            return Err(TravelError::InvalidInput);
        }
        let info = PlaceInfo {
            id: place_id.clone(), name, country_code, category, description,
            total_visitors: 0, avg_rating: 0,
            registered_by: registrar,
            registered_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::Place(place_id.clone()), &info);
        env.storage().persistent().extend_ttl(&DataKey::Place(place_id.clone()), 100_000, 200_000);
        Ok(())
    }

    pub fn get_place(env: Env, place_id: String) -> Result<PlaceInfo, TravelError> {
        env.storage().persistent()
            .get(&DataKey::Place(place_id))
            .ok_or(TravelError::PlaceNotFound)
    }

    pub fn get_place_visitors(env: Env, place_id: String) -> Vec<Address> {
        env.storage().persistent()
            .get(&DataKey::PlaceCheckIns(place_id))
            .unwrap_or(vec![&env])
    }

    // ── Check-In ──────────────────────────────────────────────────────────

    /// Provider confirms a traveler's visit.
    /// Only verified providers can stamp check-ins → prevents fake entries.
    pub fn confirm_checkin(
        env: Env,
        provider: Address,
        traveler: Address,
        place_id: String,
        note: String,
    ) -> Result<u64, TravelError> {
        provider.require_auth();
        Self::assert_initialized(&env)?;

        let prov: ProviderProfile = env.storage().persistent()
            .get(&DataKey::ProviderID(provider.clone()))
            .ok_or(TravelError::ProfileNotFound)?;
        if !prov.is_verified { return Err(TravelError::ProviderNotVerified); }

        let mut traveler_profile: TravelerProfile = env.storage().persistent()
            .get(&DataKey::TravelID(traveler.clone()))
            .ok_or(TravelError::ProfileNotFound)?;

        let mut place: PlaceInfo = env.storage().persistent()
            .get(&DataKey::Place(place_id.clone()))
            .ok_or(TravelError::PlaceNotFound)?;

        // Monotonic check-in ID
        let checkin_id: u64 = env.storage().instance().get(&ID_COUNT).unwrap_or(0) + 1;
        env.storage().instance().set(&ID_COUNT, &checkin_id);

        let record = CheckInRecord {
            id: checkin_id,
            traveler: traveler.clone(),
            place_id: place_id.clone(),
            place_name: place.name.clone(),
            country_code: place.country_code.clone(),
            confirmed_by: provider,
            visited_at: env.ledger().timestamp(),
            note,
        };

        env.storage().persistent().set(&DataKey::CheckIn(traveler.clone(), checkin_id), &record);
        env.storage().persistent().extend_ttl(&DataKey::CheckIn(traveler.clone(), checkin_id), 100_000, 200_000);

        // Append to traveler's list
        let mut clist: Vec<u64> = env.storage().persistent()
            .get(&DataKey::TravelerCheckIns(traveler.clone()))
            .unwrap_or(vec![&env]);
        clist.push_back(checkin_id);
        env.storage().persistent().set(&DataKey::TravelerCheckIns(traveler.clone()), &clist);
        env.storage().persistent().extend_ttl(&DataKey::TravelerCheckIns(traveler.clone()), 100_000, 200_000);

        // Append to place visitors
        let mut visitors: Vec<Address> = env.storage().persistent()
            .get(&DataKey::PlaceCheckIns(place_id.clone()))
            .unwrap_or(vec![&env]);
        visitors.push_back(traveler.clone());
        env.storage().persistent().set(&DataKey::PlaceCheckIns(place_id.clone()), &visitors);

        // Update traveler stats
        traveler_profile.total_checkins += 1;
        env.storage().persistent().set(&DataKey::TravelID(traveler.clone()), &traveler_profile);

        // Update place stats
        place.total_visitors += 1;
        env.storage().persistent().set(&DataKey::Place(place_id.clone()), &place);

        // Auto-badge milestones
        Self::_check_checkin_badges(&env, &traveler, traveler_profile.total_checkins);

        Ok(checkin_id)
    }

    pub fn get_checkin(env: Env, traveler: Address, checkin_id: u64) -> Result<CheckInRecord, TravelError> {
        env.storage().persistent()
            .get(&DataKey::CheckIn(traveler, checkin_id))
            .ok_or(TravelError::CheckInNotFound)
    }

    pub fn list_checkins(env: Env, traveler: Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&DataKey::TravelerCheckIns(traveler))
            .unwrap_or(vec![&env])
    }

    // ── Reviews ───────────────────────────────────────────────────────────

    /// Leave a review — MUST link to a real check-in stamped by this provider.
    /// This is the core anti-fake-review mechanism.
    pub fn leave_review(
        env: Env,
        reviewer: Address,
        provider: Address,
        checkin_id: u64,
        rating: u32,
        comment: String,
    ) -> Result<(), TravelError> {
        reviewer.require_auth();
        Self::assert_initialized(&env)?;

        if rating < 1 || rating > 5 { return Err(TravelError::InvalidRating); }

        let mut trav: TravelerProfile = env.storage().persistent()
            .get(&DataKey::TravelID(reviewer.clone()))
            .ok_or(TravelError::ProfileNotFound)?;

        let checkin: CheckInRecord = env.storage().persistent()
            .get(&DataKey::CheckIn(reviewer.clone(), checkin_id))
            .ok_or(TravelError::CheckInNotFound)?;

        if checkin.confirmed_by != provider {
            return Err(TravelError::CheckInNotConfirmed);
        }

        if env.storage().persistent().has(&DataKey::Review(reviewer.clone(), provider.clone())) {
            return Err(TravelError::ReviewAlreadyExists);
        }

        let record = ReviewRecord {
            reviewer: reviewer.clone(), provider: provider.clone(),
            checkin_id, rating, comment,
            reviewed_at: env.ledger().timestamp(),
            is_flagged: false,
        };

        env.storage().persistent().set(&DataKey::Review(reviewer.clone(), provider.clone()), &record);
        env.storage().persistent().extend_ttl(&DataKey::Review(reviewer.clone(), provider.clone()), 100_000, 200_000);

        // Update provider review list
        let mut plist: Vec<Address> = env.storage().persistent()
            .get(&DataKey::ProviderReviews(provider.clone()))
            .unwrap_or(vec![&env]);
        plist.push_back(reviewer.clone());
        env.storage().persistent().set(&DataKey::ProviderReviews(provider.clone()), &plist);

        // Recalculate provider avg rating
        let mut prov: ProviderProfile = env.storage().persistent()
            .get(&DataKey::ProviderID(provider.clone()))
            .ok_or(TravelError::ProfileNotFound)?;
        let n = prov.total_ratings + 1;
        prov.avg_rating = (prov.avg_rating * prov.total_ratings + rating * 100) / n;
        prov.total_ratings = n;
        env.storage().persistent().set(&DataKey::ProviderID(provider.clone()), &prov);

        // Update traveler stats
        trav.total_reviews += 1;
        env.storage().persistent().set(&DataKey::TravelID(reviewer.clone()), &trav);

        // TrustedReviewer badge at 20 reviews
        if trav.total_reviews >= 20 {
            Self::_try_mint_badge(
                &env, reviewer,
                BadgeType::TrustedReviewer,
                String::from_str(&env, "Người đánh giá uy tín"),
            );
        }

        Ok(())
    }

    pub fn get_review(env: Env, reviewer: Address, provider: Address) -> Result<ReviewRecord, TravelError> {
        env.storage().persistent()
            .get(&DataKey::Review(reviewer, provider))
            .ok_or(TravelError::ReviewNotFound)
    }

    pub fn list_provider_reviewers(env: Env, provider: Address) -> Vec<Address> {
        env.storage().persistent()
            .get(&DataKey::ProviderReviews(provider))
            .unwrap_or(vec![&env])
    }

    /// Admin flags a suspicious review
    pub fn flag_review(
        env: Env,
        admin: Address,
        reviewer: Address,
        provider: Address,
    ) -> Result<(), TravelError> {
        Self::assert_admin(&env, &admin)?;
        admin.require_auth();
        let mut r: ReviewRecord = env.storage().persistent()
            .get(&DataKey::Review(reviewer.clone(), provider.clone()))
            .ok_or(TravelError::ReviewNotFound)?;
        r.is_flagged = true;
        env.storage().persistent().set(&DataKey::Review(reviewer, provider), &r);
        Ok(())
    }

    // ── Badges ────────────────────────────────────────────────────────────

    pub fn get_badge(env: Env, owner: Address, badge_id: u32) -> Result<BadgeRecord, TravelError> {
        env.storage().persistent()
            .get(&DataKey::Badge(owner, badge_id))
            .ok_or(TravelError::BadgeNotFound)
    }

    pub fn list_badges(env: Env, owner: Address) -> Vec<u32> {
        env.storage().persistent()
            .get(&DataKey::OwnerBadges(owner))
            .unwrap_or(vec![&env])
    }

    /// Admin revokes a badge on fraud detection
    pub fn revoke_badge(env: Env, admin: Address, owner: Address, badge_id: u32) -> Result<(), TravelError> {
        Self::assert_admin(&env, &admin)?;
        admin.require_auth();
        let mut badge: BadgeRecord = env.storage().persistent()
            .get(&DataKey::Badge(owner.clone(), badge_id))
            .ok_or(TravelError::BadgeNotFound)?;
        badge.is_revoked = true;
        env.storage().persistent().set(&DataKey::Badge(owner, badge_id), &badge);
        Ok(())
    }

    // ── Payment Reputation ────────────────────────────────────────────────

    pub fn get_payment_rep(env: Env, owner: Address) -> Result<PaymentReputation, TravelError> {
        env.storage().persistent()
            .get(&DataKey::PaymentRep(owner))
            .ok_or(TravelError::ProfileNotFound)
    }

    /// Called by escrow/payment contract after a clean deposit+refund cycle
    pub fn record_payment_success(env: Env, admin: Address, owner: Address) -> Result<(), TravelError> {
        Self::assert_admin(&env, &admin)?;
        admin.require_auth();
        let mut rep: PaymentReputation = env.storage().persistent()
            .get(&DataKey::PaymentRep(owner.clone()))
            .ok_or(TravelError::ProfileNotFound)?;
        rep.total_deposits += 1;
        rep.successful_refunds += 1;
        rep.score = Self::_compute_payment_score(&rep);
        rep.last_updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::PaymentRep(owner.clone()), &rep);
        if rep.disputes == 0 && rep.total_deposits >= 10 {
            Self::_try_mint_badge(
                &env, owner,
                BadgeType::PaymentChampion,
                String::from_str(&env, "Thanh toán uy tín"),
            );
        }
        Ok(())
    }

    pub fn record_dispute(env: Env, admin: Address, owner: Address, won: bool) -> Result<(), TravelError> {
        Self::assert_admin(&env, &admin)?;
        admin.require_auth();
        let mut rep: PaymentReputation = env.storage().persistent()
            .get(&DataKey::PaymentRep(owner.clone()))
            .ok_or(TravelError::ProfileNotFound)?;
        rep.disputes += 1;
        if won { rep.dispute_wins += 1; }
        rep.score = Self::_compute_payment_score(&rep);
        rep.last_updated = env.ledger().timestamp();
        env.storage().persistent().set(&DataKey::PaymentRep(owner), &rep);
        Ok(())
    }

    // ── Trust Score ───────────────────────────────────────────────────────

    /// Recompute composite trust score:
    ///   40% check-in activity (capped at 400)
    ///   30% review count (capped at 300)
    ///   30% payment reputation (0–100 → 0–300)
    pub fn update_trust_score(env: Env, traveler: Address) -> Result<u32, TravelError> {
        let mut profile: TravelerProfile = env.storage().persistent()
            .get(&DataKey::TravelID(traveler.clone()))
            .ok_or(TravelError::ProfileNotFound)?;

        let rep: PaymentReputation = env.storage().persistent()
            .get(&DataKey::PaymentRep(traveler.clone()))
            .unwrap_or(PaymentReputation {
                owner: traveler.clone(), total_deposits: 0,
                successful_refunds: 0, disputes: 0,
                dispute_wins: 0, score: 100, last_updated: 0,
            });

        let checkin_pts  = (profile.total_checkins * 4).min(400);
        let review_pts   = (profile.total_reviews  * 10).min(300);
        let payment_pts  = rep.score * 3;

        profile.trust_score = checkin_pts + review_pts + payment_pts;
        env.storage().persistent().set(&DataKey::TravelID(traveler), &profile);
        Ok(profile.trust_score)
    }

    // ── Admin helpers ─────────────────────────────────────────────────────

    pub fn get_admin(env: Env) -> Result<Address, TravelError> {
        env.storage().instance().get(&ADMIN).ok_or(TravelError::NotInitialized)
    }

    pub fn transfer_admin(env: Env, current: Address, new_admin: Address) -> Result<(), TravelError> {
        Self::assert_admin(&env, &current)?;
        current.require_auth();
        env.storage().instance().set(&ADMIN, &new_admin);
        Ok(())
    }

    // ── Internal ──────────────────────────────────────────────────────────

    fn assert_admin(env: &Env, caller: &Address) -> Result<(), TravelError> {
        let admin: Address = env.storage().instance()
            .get(&ADMIN).ok_or(TravelError::NotInitialized)?;
        if &admin != caller { return Err(TravelError::Unauthorized); }
        Ok(())
    }

    fn assert_initialized(env: &Env) -> Result<(), TravelError> {
        if !env.storage().instance().has(&ADMIN) {
            return Err(TravelError::NotInitialized);
        }
        Ok(())
    }

    fn _compute_payment_score(rep: &PaymentReputation) -> u32 {
        if rep.total_deposits == 0 { return 100; }
        let base = (rep.successful_refunds * 100) / rep.total_deposits.max(1);
        let penalty = rep.disputes.saturating_sub(rep.dispute_wins) * 10;
        base.saturating_sub(penalty).min(100)
    }

    fn _mint_badge(env: &Env, owner: Address, badge_type: BadgeType, title: String) {
        let badge_id = next_badge_id(env, &owner);
        let badge = BadgeRecord {
            id: badge_id, owner: owner.clone(),
            badge_type, title,
            issued_at: env.ledger().timestamp(),
            is_revoked: false,
        };
        env.storage().persistent().set(&DataKey::Badge(owner.clone(), badge_id), &badge);
        env.storage().persistent().extend_ttl(&DataKey::Badge(owner.clone(), badge_id), 100_000, 200_000);
        let mut ids: Vec<u32> = env.storage().persistent()
            .get(&DataKey::OwnerBadges(owner.clone()))
            .unwrap_or(vec![env]);
        ids.push_back(badge_id);
        env.storage().persistent().set(&DataKey::OwnerBadges(owner), &ids);
    }

    fn _try_mint_badge(env: &Env, owner: Address, badge_type: BadgeType, title: String) {
        let ids: Vec<u32> = env.storage().persistent()
            .get(&DataKey::OwnerBadges(owner.clone()))
            .unwrap_or(vec![env]);
        for id in ids.iter() {
            if let Some(b) = env.storage().persistent()
                .get::<DataKey, BadgeRecord>(&DataKey::Badge(owner.clone(), id))
            {
                if b.badge_type == badge_type && !b.is_revoked { return; }
            }
        }
        Self::_mint_badge(env, owner, badge_type, title);
    }

    fn _check_checkin_badges(env: &Env, traveler: &Address, total: u32) {
        for &m in &[10u32, 25, 50, 100] {
            if total == m {
                Self::_try_mint_badge(
                    env, traveler.clone(),
                    BadgeType::CheckInMilestone(m),
                    String::from_str(env, "Mốc check-in"),
                );
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    fn setup() -> (Env, TravelIDContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register_contract(None, TravelIDContract);
        let client = TravelIDContractClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    fn register_place(env: &Env, client: &TravelIDContractClient, admin: &Address) {
        client.register_place(
            admin,
            &String::from_str(env, "hoi-an-001"),
            &String::from_str(env, "Hội An"),
            &String::from_str(env, "VN"),
            &String::from_str(env, "city"),
            &String::from_str(env, "Phố cổ di sản UNESCO"),
        );
    }

    #[test]
    fn test_register_traveler() {
        let (env, client, _) = setup();
        let t = Address::generate(&env);
        client.register_traveler(&t,
            &String::from_str(&env, "Nguyen Van A"),
            &String::from_str(&env, "VN"));
        let p = client.get_traveler(&t);
        assert_eq!(p.total_checkins, 0);
        assert_eq!(p.trust_score, 500);
    }

    #[test]
    fn test_full_checkin_flow() {
        let (env, client, admin) = setup();
        let traveler = Address::generate(&env);
        let provider  = Address::generate(&env);
        client.register_traveler(&traveler, &String::from_str(&env, "Du khach"), &String::from_str(&env, "VN"));
        client.register_provider(&provider, &String::from_str(&env, "Hoi An Tours"),
            &String::from_str(&env, "tour"), &String::from_str(&env, "VN"),
            &String::from_str(&env, "LIC-001"));
        client.verify_provider(&admin, &provider);
        register_place(&env, &client, &admin);

        let id = client.confirm_checkin(
            &provider, &traveler,
            &String::from_str(&env, "hoi-an-001"),
            &String::from_str(&env, "Tuyet voi!"),
        );
        assert_eq!(id, 1);

        let p = client.get_traveler(&traveler);
        assert_eq!(p.total_checkins, 1);
    }

    #[test]
    fn test_review_requires_real_checkin() {
        let (env, client, _) = setup();
        let traveler = Address::generate(&env);
        let provider  = Address::generate(&env);
        client.register_traveler(&traveler, &String::from_str(&env, "T"), &String::from_str(&env, "VN"));
        client.register_provider(&provider, &String::from_str(&env, "P"),
            &String::from_str(&env, "tour"), &String::from_str(&env, "VN"),
            &String::from_str(&env, "X"));
        let r = client.try_leave_review(
            &traveler, &provider, &0u64, &5u32,
            &String::from_str(&env, "Gia mao"),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_unverified_provider_blocked() {
        let (env, client, admin) = setup();
        let traveler = Address::generate(&env);
        let provider  = Address::generate(&env);
        client.register_traveler(&traveler, &String::from_str(&env, "T"), &String::from_str(&env, "VN"));
        client.register_provider(&provider, &String::from_str(&env, "Chua xac minh"),
            &String::from_str(&env, "tour"), &String::from_str(&env, "VN"),
            &String::from_str(&env, "NONE"));
        register_place(&env, &client, &admin);
        let r = client.try_confirm_checkin(
            &provider, &traveler,
            &String::from_str(&env, "hoi-an-001"),
            &String::from_str(&env, ""),
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_payment_reputation_score() {
        let (env, client, admin) = setup();
        let user = Address::generate(&env);
        client.register_traveler(&user, &String::from_str(&env, "U"), &String::from_str(&env, "VN"));
        client.record_payment_success(&admin, &user);
        let rep = client.get_payment_rep(&user);
        assert_eq!(rep.total_deposits, 1);
        assert_eq!(rep.score, 100);
    }

    #[test]
    fn test_trust_score_computation() {
        let (env, client, admin) = setup();
        let traveler = Address::generate(&env);
        let provider  = Address::generate(&env);
        client.register_traveler(&traveler, &String::from_str(&env, "T"), &String::from_str(&env, "VN"));
        client.register_provider(&provider, &String::from_str(&env, "P"),
            &String::from_str(&env, "tour"), &String::from_str(&env, "VN"),
            &String::from_str(&env, "LIC"));
        client.verify_provider(&admin, &provider);
        register_place(&env, &client, &admin);
        client.confirm_checkin(&provider, &traveler,
            &String::from_str(&env, "hoi-an-001"),
            &String::from_str(&env, ""));
        let score = client.update_trust_score(&traveler);
        // 1 checkin * 4 = 4 pts + 0 reviews + 100*3 = 300 payment pts = 304
        assert_eq!(score, 304);
    }
}
