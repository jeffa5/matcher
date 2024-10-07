# Matcher

A service to provide matchings between people, intended for meeting others within an organisation.

## Service

Provides:
- way to register new people
- stats per person, including history of matching and current weights with people
- Function to clear current matches (e.g. before the next round of matching)
- Generating matching between all registered people
- Function to track whether people met, to ensure accuracy in the weights for next time

### Functions

- POST `/people`
    - json body `{"name": "john smith", "email": "john@smith.net"}`

- GET `/people`

- GET `/matching`
