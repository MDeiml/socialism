# Socialism

Free time is common property.

## API

For all API calls except `POST /user`, `POST /session` and `DELETE /session` `?token=<token>` has to be appended to the URL where `<token>`is the token returned from `POST /session`.

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
    * `POST {username: String, password: String} -> String`: Log in. Returns UNAUTHORIZED if user and password do not match or user does not exist. Otherwise returns a session token.
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

