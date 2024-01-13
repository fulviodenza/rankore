# rankore
A discord bot to track user activity in a Discord server.
This bot is currently tracking voice and text user activity into a Discord server.<br>

[Invite me!](https://discord.com/oauth2/authorize?client_id=1161409490369912924&permissions=8&scope=bot)
 or Join the official [rankore Discord server](https://discord.gg/jfSvzPDY)!
 
## Commands
- `!leaderboard`: List the users and their points, from the most active to the less active;
- `!set_prefix [PREFIX]`: Set the prefix for the Discord server in which the bot is running; (After this command the default `!` prefix will not be active, replaced by the one you set)
- `!set_welcome_message [STRING]`: Set the welcome message
- `!help`: Get this help message
- `!reset_scores`: Reset leaderboard scores
- `!set_voice_multiplier [INTEGER]`: set the multiplier to calculate the points for the voice activity for a user in a Discord server. greater the multiplier, greater will be the wait to add a point to that user. For example, if the admin sets the multiplier to 5, the bot will wait 5 seconds before incrementing 1 point to the user
- `!set_text_multiplier [INTEGER]`: set the multiplier for each message, this is simply the points for each message
- `!multipliers`: shows the `set_voice_multiplier` and the `set_text_multiplier`

Ensure to have a `#welcome` channel existing in the Discord server!