//! Where Jekyll publishes a page, a post or a document of a collection, by
//! Jekyll's own rules: a template of `:placeholders`, filled in and
//! tidied as Jekyll tidies it.

use crate::date::Date;

/// The settings' `permalink`, as Jekyll reads it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Style {
    /// One of Jekyll's named styles: `date`, `pretty`, `ordinal`,
    /// `weekdate` or `none`.
    Named(&'static str),
    Pattern(String),
}

impl Style {
    /// The style a setting names; `date` where there is none.
    pub(super) fn of(setting: Option<&str>) -> Style {
        match setting.unwrap_or("date") {
            "date" => Style::Named("date"),
            "pretty" => Style::Named("pretty"),
            "ordinal" => Style::Named("ordinal"),
            "weekdate" => Style::Named("weekdate"),
            "none" => Style::Named("none"),
            pattern => Style::Pattern(pattern.to_owned()),
        }
    }

    /// The template a post's address is filled in from.
    pub(super) fn post_template(&self) -> String {
        match self {
            Style::Named("pretty") => "/:categories/:year/:month/:day/:title/".into(),
            Style::Named("ordinal") => "/:categories/:year/:y_day/:title:output_ext".into(),
            Style::Named("weekdate") => {
                "/:categories/:year/W:week/:short_day/:title:output_ext".into()
            }
            Style::Named("none") => "/:categories/:title:output_ext".into(),
            Style::Named(_) => "/:categories/:year/:month/:day/:title:output_ext".into(),
            Style::Pattern(pattern) => pattern.clone(),
        }
    }

    /// `template` with what the style ends an address with: a `/` for
    /// `pretty` or a pattern ending in one, the extension for the other
    /// named styles or a pattern ending in `:output_ext`.
    pub(super) fn suffixed(&self, template: &str) -> String {
        match self {
            Style::Named("pretty") => format!("{template}/"),
            Style::Named(_) => format!("{template}:output_ext"),
            Style::Pattern(pattern) if pattern.ends_with('/') => format!("{template}/"),
            Style::Pattern(pattern) if pattern.ends_with(":output_ext") => {
                format!("{template}:output_ext")
            }
            Style::Pattern(_) => template.to_owned(),
        }
    }
}

/// `template` filled in from `value`, which gives a placeholder's value
/// by its name, then tidied: `//` made one and the address opened with
/// `/`. The unknown placeholder where `value` has none.
pub(super) fn fill(
    template: &str,
    value: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(colon) = rest.find(':') {
        out.push_str(&rest[..colon]);
        let after = &rest[colon + 1..];
        let length = after
            .find(|c: char| !(c.is_ascii_lowercase() || c == '_'))
            .unwrap_or(after.len());
        if length == 0 {
            out.push(':');
            rest = after;
            continue;
        }
        let name = &after[..length];
        let (filled, used) = match value(name) {
            Some(filled) => (filled, length),
            None => match name
                .strip_suffix('_')
                .and_then(|chomped| Some((value(chomped)?, length - 1)))
            {
                Some(found) => found,
                None => return Err(format!(":{name}")),
            },
        };
        out.push_str(&filled);
        rest = &after[used..];
    }
    out.push_str(rest);
    let mut tidy = format!("/{out}").replace("..", "/").replace("./", "");
    while tidy.contains("//") {
        tidy = tidy.replace("//", "/");
    }
    Ok(tidy)
}

/// A name made ready for an address as Jekyll's `slugify` makes it: every
/// run of characters other than letters and figures, and in its `pretty`
/// mode also `._~!$&'()+,;=@`, made one hyphen, with none at either end,
/// and in small letters unless `cased`.
pub(super) fn slugify(text: &str, pretty: bool, cased: bool) -> String {
    let kept = |c: char| c.is_alphanumeric() || (pretty && "._~!$&'()+,;=@".contains(c));
    let mut out = String::with_capacity(text.len());
    let mut gap = false;
    for c in text.chars() {
        if kept(c) {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    match cased {
        true => out,
        false => out.to_lowercase(),
    }
}

/// A moment's pieces as Jekyll writes them into an address.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) struct Moment {
    date: jiff::civil::Date,
    hour: i8,
    minute: i8,
    second: i8,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl Moment {
    /// The moment `date` names, on the clock of `zone` where the date has
    /// an offset and the site a zone, as Jekyll reads it there; else as
    /// written.
    pub(super) fn of(date: &Date, zone: Option<&str>) -> Option<Moment> {
        let time = date.time;
        let (hour, minute, second) = match time {
            Some(time) => (time.hour, time.minute, time.second.unwrap_or(0)),
            None => (0, 0, 0),
        };
        let civil = jiff::civil::Date::new(
            i16::try_from(date.year).ok()?,
            i8::try_from(date.month).ok()?,
            i8::try_from(date.day).ok()?,
        )
        .ok()?;
        let written = Moment {
            date: civil,
            hour: i8::try_from(hour).ok()?,
            minute: i8::try_from(minute).ok()?,
            second: i8::try_from(second).ok()?,
        };
        let (Some(offset), Some(zone)) = (date.offset, zone) else {
            return Some(written);
        };
        let at = civil.at(written.hour, written.minute, written.second, 0);
        let offset = jiff::tz::Offset::from_seconds(i32::from(offset) * 60).ok()?;
        let zone = jiff::tz::TimeZone::get(zone).ok()?;
        let there = offset.to_timestamp(at).ok()?.to_zoned(zone).datetime();
        Some(Moment {
            date: there.date(),
            hour: there.hour(),
            minute: there.minute(),
            second: there.second(),
        })
    }

    /// The value of one of Jekyll's date placeholders, by its name.
    pub(super) fn piece(&self, name: &str) -> Option<String> {
        let d = self.date;
        let month = MONTHS[usize::try_from(d.month() - 1).ok()?];
        let weekday = DAYS[usize::try_from(d.weekday().to_monday_zero_offset()).ok()?];
        let week = d.iso_week_date();
        Some(match name {
            "year" => format!("{:04}", d.year()),
            "month" => format!("{:02}", d.month()),
            "day" => format!("{:02}", d.day()),
            "i_month" => d.month().to_string(),
            "i_day" => d.day().to_string(),
            "short_year" => format!("{:02}", d.year().rem_euclid(100)),
            "y_day" => format!("{:03}", d.day_of_year()),
            "hour" => format!("{:02}", self.hour),
            "minute" => format!("{:02}", self.minute),
            "second" => format!("{:02}", self.second),
            "short_month" => month[..3].to_owned(),
            "long_month" => month.to_owned(),
            "short_day" => weekday[..3].to_owned(),
            "long_day" => weekday.to_owned(),
            "week" => format!("{:02}", week.week()),
            "w_year" => week.year().to_string(),
            "w_day" => (d.weekday().to_monday_one_offset()).to_string(),
            _ => return None,
        })
    }
}
