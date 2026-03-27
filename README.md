TravelID is a decentralized identity and reputation system for travelers and tourism service providers built on the Stellar Soroban smart contract platform.

The system allows travelers to build a verifiable travel identity ("Travel Passport") on-chain through confirmed visits, reviews, and reputation records. Service providers such as hotels, tour operators, and guides can verify traveler visits and receive trusted feedback.

The primary goal of this contract is to reduce fake reviews, increase trust in tourism platforms, and provide a transparent reputation system using blockchain technology.

This smart contract manages:

Traveler identity profiles
Tourism service provider profiles
Verified travel check-ins
Authentic review system
Achievement badges (soulbound NFTs)
Payment reputation scores
Destination/place registry
Trust score calculation

All records are stored on-chain and tamper-resistant.

Key Features
1. Traveler On-Chain Identity

Travelers can register a TravelerProfile which acts as a digital travel passport.

Each profile stores:

Display name
Country code
Number of verified check-ins
Number of reviews written
Countries visited
Trust score
Registration timestamp

This allows users to build a verifiable travel history.

2. Verified Service Providers

Tourism businesses can register as providers, including:

Hotels
Tour operators
Local guides
Restaurants

Providers must be verified by the system administrator before they can confirm traveler check-ins.

Verification includes:

License validation
Provider badge minting

This prevents malicious actors from confirming fake visits.

3. Verified Travel Check-Ins

A traveler’s visit to a location must be confirmed by a verified provider.

Each check-in record contains:

Traveler address
Place ID
Place name
Country code
Provider who confirmed the visit
Visit timestamp
Optional note

This ensures that travel history is legitimate and cannot be faked.

4. Anti-Fake Review System

Reviews can only be submitted if:

The traveler has a real check-in
The check-in was confirmed by the same provider

This eliminates common problems in travel platforms such as:

Fake reviews
Bot reviews
Competitor manipulation

Providers maintain:

Average rating
Total rating count

Ratings are stored as rating × 100 to avoid floating-point calculations.

5. Achievement Badge System

The contract includes Soulbound badges, meaning they cannot be transferred.

Badges are awarded automatically for achievements such as:

Check-in milestones (10, 25, 50, 100)
Trusted reviewer status
Verified provider status
Excellent payment reputation
Explorer achievements

Badges help create a gamified travel reputation system.

6. Payment Reputation

The contract tracks payment reliability for users.

Metrics include:

Total deposits
Successful refunds
Disputes
Disputes won

A payment score (0-100) is calculated to represent user trustworthiness in financial transactions.

Users with strong payment records can receive the Payment Champion badge.

7. Destination Registry

Locations such as cities, landmarks, beaches, or mountains can be registered on-chain.

Each destination stores:

Unique ID
Name
Country code
Category
Description
Total visitors
Average rating

This allows the system to track global tourism activity on-chain.

8. Trust Score Algorithm

Each traveler has a composite trust score (0-1000).

The score is calculated using three components:

Component	Weight
Check-ins	40%
Reviews	30%
Payment reputation	30%

Formula concept:

Trust Score =
Check-in Activity +
Review Activity +
Payment Reputation

This score helps determine user credibility in the ecosystem.
