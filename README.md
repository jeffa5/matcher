# Matcher

A service to provide matchings between people, intended for meeting others within an organisation.

## Service

Provides:
- way to register new people
- stats per person, including history of matching and current weights with people
- Function to clear current matches (e.g. before the next round of matching)
- Generating matching between all registered people
- Function to track whether people met, to ensure accuracy in the weights for next time

### Sign up for this round

A person wants to sign up for the matching round.
- pass in name and email
- ensure person is in `persons` table
- write new person id into `waiting` table

### View person

A person wants to see their history of matches along with their email
- pass in email
- check if person is in `persons` table
- filter `matches` table to those including person id
- find if person id is in `waiting` table so they can mutate it

### View matchings

A person (admin) wants to view all of the current matchings
- filter `matches` table to latest round of matching and return them for viewing

### Trigger matching

An admin wants to create a new set of matchings
- obtain the list of people for this matching round using `waiting` table
- build the graph from the `edges` table which has the weights for edges between people
    - filter edges down to those where both ends are in the `waiting` list
- run matching
- write matching to `matches` table, update edge counts in `edges` table, delete `waiting` people

## Data model

Person: id, name, email
Matching: generation, person1.id, optional person2.id
Generations: generation, time
Edges: person1.id, person2.id, weight
Waiting: person.id

## Auth

Signing up is just creating a user and providing the token for the instance, then users get a unique token of their own to manage their page.

Conveniently given as a link as well as the token for just signing in.

Once signed in you can view every person and all matchings, (you are trusted), but can only edit your own person page.
