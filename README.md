## API

All API calls except `register` and `login` have an additional session parameter.

### User

* `register(username: String, password: String)`
* `login(username: String, password: String) -> Session`
* `logout()`

### Blocked Time

Suggestions are open for a better name

* `add(start: Time, end: Time, repeat: DAILY | WEEKLY | NO_REPEAT)`
* `remove(start: Time, end: Time)`
* `list() -> [(start: Time, end: Time)]`

### Group

* `create(name: String) -> id: GroupId`
* `add_user(group: GroupId, username: String)`
* `remove_user(group: GroupId, username: String)`
* `list() -> [id: GroupId]`
* `make_admin(group: GroupId, username: String)`

### Activity

Later we should add funcionality to suggest multiple options for start and end.
Should you be able to see who accepted?
Should we add another status `MAYBE_ACCEPTED` for when someone accepts multiple activities for the same timeframe?
Should admins be able to cancel activities?
Should activities be canceled atomatically if there is no one with `ACCEPTED` status? 
Should you be able to change your status if the activity is `SUCCEDED`?

* `create(group: GroupId, start: Time, end: Time, description: String, min_participants: Int, max_participants: Int) -> ActivityId`
* `get(id: ActivityId) -> (group: GroupId, start: Time, end: Time, description: String, min_participants: Int, max_participants: Int, accepted: Int, pending: Int)`
* `list() -> [(id: ActivityId, status: PENDING | ACCEPTED | DENIED | SUCCEDED)]`
* `change_status(id: ActivityId, status: PENDING | ACCEPTED | DENIED)`
