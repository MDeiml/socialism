# Socialism

Free time is common property.

## API

All API calls except `register` and `login` have an additional session parameter.

### Types

* `Block {start: int, end: int}`: A time interval. Used for blocked time and activities.
* `User {username: String, blocks: [Block]}`: A user. Does not include password data.
* `Group {name: String, users: {user_id: is_admin}}`: A group of users.
* `Activity {group_id: int, block: Block, description: String, min_participants: int, max_participants: int, accepted: int, pending: int}`: An activity. When posting the `accepted` and `pending` fields are optional and will be ignored.
* `Status "Accepted" | "Pending" | "Denied"`

### Routes

* `/user`
    * `POST {username: String, password: String}`: Register a new user. Returns CONFLICT if user already exists.
    * `GET -> User`: Get current logged in user
* `/session`
    * `POST {username: String, password: String}`: Log in. Returns UNAUTHORIZED if user and password do not match or user does not exist.
    * `DELETE`: Log out.
* `/block`
    * `POST Block`: Add new blocked time. Returns CONFLICT if this intersects another blocked time for this user.
    * `POST Block`: Remove blocked time. Returns NOT FOUDN if there is no such blocked time for this user.
* `/group`
    * `POST String -> group_id`: Create a new group with the given name. The current user is automatically added as a group admin.
    * `GET -> {group_id: Group}`: List all groups for the current user.
* `/group/user`
    * `POST {group_id: int, user_id: int}`: Add a user to a group. Returns NOT FOUND if the logged in user is not a member of this group. Returns FORBIDDEN if the logged in user is not an admin of this group.
    * `DELETE {group_id: int, user_id: int}`: Remove a user from a group. Returns NOT FOUND if the logged in user is not a member of this group. Returns FORBIDDEN if the logged in user is not equal to the given user and the logged in user is not an admin of this group.
* `/group/admin`
    * `POST {group_id: int, user_id: int}`: Promote a user to admin. Returns NOT FOUND if the logged in user is not a member of this group. Returns FORBIDDEN if the logged in user is not an admin of this group.
* `/activity`
    * `POST Activity -> activity_id`: Create a new activity. Returns NOT FOUND if the logged in user is not a member of this group.
    * `GET -> {activity_id: {activity: Activity, status: Status}}`: List all activities for all groups of the current user.
* `/activity/status`
    * `POST {activity_id: int, status: Status}"`: Set this users status for the given activity. Returns NOT FOUND if the logged in user is not a member of this group.

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
