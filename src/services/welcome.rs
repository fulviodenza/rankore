use serenity::all::{Context, Member};

/// Substitute supported placeholders in a welcome template. Supported tokens:
///   {user}          — mention of the new member
///   {username}      — plain display name (no mention)
///   {server}        — guild name
///   {member_count}  — current member count (best-effort from the cache)
///
/// Backwards compatibility: if the template contains none of the tokens
/// above, the legacy "<msg>, <@user>!" form is produced so existing
/// configurations keep working unchanged.
pub async fn render(ctx: &Context, template: &str, member: &Member) -> String {
    let has_token = template.contains("{user}")
        || template.contains("{username}")
        || template.contains("{server}")
        || template.contains("{member_count}");
    if !has_token {
        return format!("{}, <@{}>!", template, member.user.id);
    }

    let user_mention = format!("<@{}>", member.user.id);
    let username = member
        .nick
        .clone()
        .unwrap_or_else(|| member.display_name().to_string());
    let (server_name, member_count) = match ctx.cache.guild(member.guild_id) {
        Some(g) => (g.name.clone(), g.member_count),
        None => (String::new(), 0),
    };

    template
        .replace("{user}", &user_mention)
        .replace("{username}", &username)
        .replace("{server}", &server_name)
        .replace("{member_count}", &member_count.to_string())
}
