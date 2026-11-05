use std::collections::BTreeMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, ImportMode, ImportReport, QuizQuestion};
use crate::auth::use_auth;
use crate::routes::Route;
use crate::components::button::{Button, ButtonVariant};

const QUIZ_CSS: Asset = asset!("/assets/quiz.css");
const LETTERS: [&str; 8] = ["A", "B", "C", "D", "E", "F", "G", "H"];
/// Matches the API parser's fallback when a question has no category.
const DEFAULT_CATEGORY: &str = "Uncategorized";
/// The bank list only renders this many matches; the search narrows it down.
const MAX_LISTED: usize = 150;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Quiz,
    Bank,
    Import,
}

#[derive(Clone, PartialEq)]
enum Filter {
    All,
    /// Text before the first `/` of the category, e.g. `Rootkits`.
    Top(String),
    /// Exact category, launched from the bank's category table.
    Category(String),
    Unseen,
    Mistakes,
}

impl Filter {
    fn matches(&self, q: &QuizQuestion) -> bool {
        match self {
            Filter::All => true,
            Filter::Top(top) => top_category(&q.category) == top,
            Filter::Category(category) => &q.category == category,
            Filter::Unseen => q.last_correct.is_none(),
            Filter::Mistakes => q.last_correct == Some(false),
        }
    }

    fn label(&self) -> String {
        match self {
            Filter::All => "All".to_string(),
            Filter::Top(name) | Filter::Category(name) => name.clone(),
            Filter::Unseen => "Never seen".to_string(),
            Filter::Mistakes => "My mistakes".to_string(),
        }
    }
}

/// An option as displayed: shuffled and relabelled A, B, C… by position,
/// remembering its original letter for the server.
#[derive(Clone, PartialEq)]
struct ShownOption {
    letter: String,
    original: String,
    text: String,
    detail: String,
}

#[derive(Clone, PartialEq)]
struct Item {
    id: i64,
    num: i64,
    category: String,
    prompt: String,
    options: Vec<ShownOption>,
    /// Displayed letters of the correct options, sorted.
    correct: Vec<String>,
}

impl Item {
    fn is_multi(&self) -> bool {
        self.correct.len() > 1
    }
}

#[derive(Clone, PartialEq)]
struct Session {
    filter: Filter,
    items: Vec<Item>,
    index: usize,
    score: usize,
    answered: bool,
    /// Displayed letters picked for the current item.
    selected: Vec<String>,
    results: Vec<(Item, bool)>,
}

/// xorshift64*, seeded from the clock — enough to shuffle a quiz without
/// pulling in a `rand` dependency for the wasm build.
struct Rng(u64);

impl Rng {
    fn seeded() -> Self {
        // splitmix64 step so close timestamps still give unrelated seeds.
        let mut z = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Self((z ^ (z >> 31)) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

fn top_category(category: &str) -> &str {
    let top = category.split('/').next().unwrap_or("").trim();
    if top.is_empty() { DEFAULT_CATEGORY } else { top }
}

/// Lowercase without common Latin accents, for accent-insensitive search and sort.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            other => other,
        })
        .collect()
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn shuffle_options(q: &QuizQuestion, rng: &mut Rng) -> Item {
    let mut options = q.options.clone();
    rng.shuffle(&mut options);
    let options: Vec<ShownOption> = options
        .into_iter()
        .zip(LETTERS)
        .map(|(o, letter)| ShownOption { letter: letter.to_string(), original: o.letter, text: o.text, detail: o.detail })
        .collect();
    let correct = options.iter().filter(|o| q.correct.contains(&o.original)).map(|o| o.letter.clone()).collect();

    Item { id: q.id, num: q.num, category: q.category.clone(), prompt: q.prompt.clone(), options, correct }
}

/// Picks up to `count` questions (0 = all) matching `filter`, in random
/// order. Falls back to the whole bank if nothing matches.
fn start_session(bank: &[QuizQuestion], filter: Filter, count: usize) -> Option<Session> {
    let mut filter = filter;
    let mut pool: Vec<&QuizQuestion> = bank.iter().filter(|q| filter.matches(q)).collect();
    if pool.is_empty() {
        filter = Filter::All;
        pool = bank.iter().collect();
    }
    if pool.is_empty() {
        return None;
    }

    let mut rng = Rng::seeded();
    rng.shuffle(&mut pool);
    if count > 0 {
        pool.truncate(count);
    }
    let items = pool.into_iter().map(|q| shuffle_options(q, &mut rng)).collect();

    Some(Session { filter, items, index: 0, score: 0, answered: false, selected: Vec::new(), results: Vec::new() })
}

#[component]
pub fn Quiz() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    let bank_resource = use_resource(move || async move {
        let token = auth.token()?;
        api::get_quiz_questions(&token).await.ok()
    });

    // Local copy of the bank so answers can update progress optimistically
    // without refetching. `None` until the first load settles, then whether
    // it succeeded.
    let mut bank = use_signal(Vec::<QuizQuestion>::new);
    let mut loaded = use_signal(|| None::<bool>);
    let mut session = use_signal(|| None::<Session>);
    let count = use_signal(|| 20usize);
    let mut tab = use_signal(|| Tab::Quiz);

    use_effect(move || match &*bank_resource.read() {
        Some(Some(questions)) => {
            bank.set(questions.clone());
            loaded.set(Some(true));
            // Any reload follows an import, a deletion or a reset: start over.
            session.set(None);
        }
        Some(None) => loaded.set(Some(false)),
        None => {}
    });

    use_effect(move || {
        if session.read().is_none() && !bank.read().is_empty() {
            let next = start_session(&bank.read(), Filter::All, *count.peek());
            session.set(next);
        }
    });

    let total = bank.read().len();
    let mastered = bank.read().iter().filter(|q| q.last_correct == Some(true)).count();

    rsx! {
        document::Link { rel: "stylesheet", href: QUIZ_CSS }
        div { class: "quiz-page",
            div { class: "quiz-header",
                div {
                    h1 { "Quiz" }
                    if total > 0 {
                        p { class: "quiz-muted", "Mastered: " strong { "{mastered}" } " / {total}" }
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| { nav.push(Route::Home {}); },
                    "Back home"
                }
            }

            nav { class: "quiz-tabs",
                for (value, label) in [(Tab::Quiz, "Quiz"), (Tab::Bank, "Bank"), (Tab::Import, "Import")] {
                    button {
                        r#type: "button",
                        "aria-current": if tab() == value { "page" } else { "false" },
                        onclick: move |_| tab.set(value),
                        "{label}"
                    }
                }
            }

            match loaded() {
                None => rsx! { p { "Loading..." } },
                Some(false) => rsx! { p { "Unable to load your quiz bank." } },
                Some(true) => match tab() {
                    Tab::Quiz => rsx! { QuizTab { bank, session, count, tab } },
                    Tab::Bank => rsx! { BankTab { bank, bank_resource, session, count, tab } },
                    Tab::Import => rsx! { ImportTab { bank, bank_resource, tab } },
                },
            }
        }
    }
}

#[component]
fn QuizTab(bank: Signal<Vec<QuizQuestion>>, session: Signal<Option<Session>>, count: Signal<usize>, tab: Signal<Tab>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let mut container = use_signal(|| None::<Rc<MountedData>>);

    // Options get disabled once answered, which drops focus to <body>; pull
    // it back so the keyboard shortcuts keep working.
    let focus = move || {
        if let Some(element) = container() {
            spawn(async move {
                let _ = element.set_focus(true).await;
            });
        }
    };

    let mut restart = move |filter: Filter| {
        let next = start_session(&bank.read(), filter, count());
        session.set(next);
        focus();
    };

    let mut validate = move || {
        let Some(mut s) = session() else { return };
        if s.answered || s.selected.is_empty() {
            return;
        }
        let Some(item) = s.items.get(s.index).cloned() else { return };

        let mut chosen = s.selected.clone();
        chosen.sort();
        let ok = chosen == item.correct;
        s.answered = true;
        if ok {
            s.score += 1;
        }
        s.results.push((item.clone(), ok));
        session.set(Some(s));

        if let Some(q) = bank.write().iter_mut().find(|q| q.id == item.id) {
            q.attempts += 1;
            if ok {
                q.successes += 1;
            }
            q.last_correct = Some(ok);
        }

        let originals: Vec<String> = item
            .options
            .iter()
            .filter(|o| chosen.contains(&o.letter))
            .map(|o| o.original.clone())
            .collect();
        if let Some(token) = auth.token() {
            spawn(async move {
                if let Err(message) = api::submit_quiz_attempt(item.id, &token, &originals).await {
                    toast.error(format!("Your answer couldn't be saved: {message}"), ToastOptions::new());
                }
            });
        }
        focus();
    };

    let mut pick = move |letter: String| {
        let Some(mut s) = session() else { return };
        if s.answered {
            return;
        }
        let Some(item) = s.items.get(s.index) else { return };
        if !item.options.iter().any(|o| o.letter == letter) {
            return;
        }
        if item.is_multi() {
            if let Some(pos) = s.selected.iter().position(|l| *l == letter) {
                s.selected.remove(pos);
            } else {
                s.selected.push(letter);
            }
            session.set(Some(s));
        } else {
            s.selected = vec![letter];
            session.set(Some(s));
            validate();
        }
    };

    let mut next = move || {
        if let Some(s) = session.write().as_mut() {
            s.index += 1;
            s.answered = false;
            s.selected.clear();
        }
        focus();
    };

    let on_key = move |e: KeyboardEvent| {
        let modifiers = e.modifiers();
        if modifiers.ctrl() || modifiers.meta() || modifiers.alt() {
            return;
        }
        let Some(s) = session() else { return };
        if s.index >= s.items.len() {
            return;
        }
        match e.key() {
            Key::Enter => {
                // Also stops Enter on a focused option button from clicking it.
                e.prevent_default();
                if s.answered {
                    next();
                } else if s.items[s.index].is_multi() {
                    validate();
                }
            }
            Key::Character(c) if !s.answered => {
                let letter = match c.as_str() {
                    d @ ("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8") => d.parse::<usize>().ok().map(|n| LETTERS[n - 1].to_string()),
                    l if l.len() == 1 && LETTERS.contains(&l.to_uppercase().as_str()) => Some(l.to_uppercase()),
                    _ => None,
                };
                if let Some(letter) = letter {
                    e.prevent_default();
                    pick(letter);
                }
            }
            _ => {}
        }
    };

    let questions = bank.read();
    if questions.is_empty() {
        return rsx! {
            div { class: "quiz-notice",
                "No questions yet. "
                button { class: "quiz-link", r#type: "button", onclick: move |_| tab.set(Tab::Import), "Import your quizzes" }
                " to get started."
            }
        };
    }

    let current_filter = session.read().as_ref().map(|s| s.filter.clone()).unwrap_or(Filter::All);
    let mut tops: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for q in questions.iter() {
        let top = top_category(&q.category);
        tops.entry(fold(top)).or_insert_with(|| (top.to_string(), 0)).1 += 1;
    }
    let mut chips: Vec<(Filter, usize)> = vec![(Filter::All, questions.len())];
    if let Filter::Category(_) = &current_filter {
        chips.push((current_filter.clone(), questions.iter().filter(|q| current_filter.matches(q)).count()));
    }
    chips.extend(tops.into_values().map(|(name, n)| (Filter::Top(name), n)));
    for filter in [Filter::Unseen, Filter::Mistakes] {
        let n = questions.iter().filter(|q| filter.matches(q)).count();
        chips.push((filter, n));
    }
    let mistakes = questions.iter().filter(|q| q.last_correct == Some(false)).count();
    drop(questions);

    rsx! {
        div { class: "quiz-toolbar",
            div { class: "quiz-filters", role: "group", "aria-label": "Choose questions",
                for (filter, n) in chips {
                    button {
                        class: "quiz-chip",
                        r#type: "button",
                        "aria-pressed": "{filter == current_filter}",
                        disabled: n == 0,
                        onclick: {
                            let filter = filter.clone();
                            move |_| restart(filter.clone())
                        },
                        "{filter.label()}"
                        span { class: "quiz-chip-count", "{n}" }
                    }
                }
            }
            label { class: "quiz-muted",
                "Questions per round "
                select {
                    class: "quiz-select",
                    onchange: move |e: FormEvent| {
                        count.set(e.value().parse().unwrap_or(0));
                        restart(current_filter.clone());
                    },
                    for (value, label) in [(10, "10"), (20, "20"), (50, "50"), (0, "All")] {
                        option { value: "{value}", selected: count() == value, "{label}" }
                    }
                }
            }
        }

        div {
            class: "quiz-stage",
            tabindex: "0",
            onmounted: move |e| {
                let element = e.data();
                container.set(Some(element.clone()));
                spawn(async move {
                    let _ = element.set_focus(true).await;
                });
            },
            onkeydown: on_key,

            match session() {
                None => rsx! {},
                Some(s) if s.index >= s.items.len() => rsx! {
                    EndCard { session: s, mistakes, on_restart: restart }
                },
                Some(s) => {
                    let item = s.items[s.index].clone();
                    let total = s.items.len();
                    let progress = 100 * s.index / total;
                    let multi = item.is_multi();
                    let is_last = s.index + 1 >= total;
                    rsx! {
                        div { class: "quiz-card",
                            div { class: "quiz-meta",
                                span { "{item.category} · Quiz {item.num}" }
                                span { "Question {s.index + 1} / {total} · score {s.score}" }
                            }
                            div { class: "quiz-progress", i { style: "width: {progress}%;" } }
                            p { class: "quiz-prompt", Rich { text: item.prompt.clone() } }
                            if multi {
                                span { class: "quiz-multi-hint", "Several correct answers: select them, then submit" }
                            }
                            div { class: "quiz-options",
                                for option in item.options.clone() {
                                    OptionButton {
                                        key: "{item.id}-{option.letter}",
                                        selected: s.selected.contains(&option.letter),
                                        state: if !s.answered {
                                            OptionState::Open
                                        } else {
                                            match (item.correct.contains(&option.letter), s.selected.contains(&option.letter)) {
                                                (true, true) => OptionState::Right,
                                                (true, false) => OptionState::Missed,
                                                (false, true) => OptionState::Wrong,
                                                (false, false) => OptionState::Neutral,
                                            }
                                        },
                                        multi,
                                        on_pick: pick,
                                        option,
                                    }
                                }
                            }
                            if s.answered {
                                {
                                    let ok = s.results.last().is_some_and(|(_, ok)| *ok);
                                    let label = if multi { "Correct answers" } else { "Correct answer" };
                                    let letters = item.correct.join(", ");
                                    rsx! {
                                        div { class: if ok { "quiz-feedback ok" } else { "quiz-feedback ko" },
                                            if ok {
                                                b { "Correct." }
                                                "Each answer's explanation is shown above."
                                            } else {
                                                b { "Wrong. {label}: {letters}" }
                                                if multi {
                                                    "Read the explanation under each answer: correct ones are green, the ones you missed are dashed."
                                                } else {
                                                    "Read the explanation under each answer: the correct one is green."
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "quiz-actions",
                                span { class: "quiz-hint",
                                    if multi { "Letters to tick, Enter to submit" } else { "Letter or number to answer, Enter to continue" }
                                }
                                if s.answered {
                                    Button { onclick: move |_| next(), if is_last { "See results" } else { "Next" } }
                                } else if multi {
                                    Button { disabled: s.selected.is_empty(), onclick: move |_| validate(), "Submit" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum OptionState {
    Open,
    Right,
    Wrong,
    Missed,
    Neutral,
}

#[component]
fn OptionButton(option: ShownOption, state: OptionState, selected: bool, multi: bool, on_pick: EventHandler<String>) -> Element {
    let class = match state {
        OptionState::Open => "quiz-option",
        OptionState::Right => "quiz-option right",
        OptionState::Wrong => "quiz-option wrong",
        OptionState::Missed => "quiz-option missed",
        OptionState::Neutral => "quiz-option neutral",
    };
    let answered = state != OptionState::Open;
    let letter = option.letter.clone();

    rsx! {
        button {
            class,
            r#type: "button",
            disabled: answered,
            "aria-pressed": if multi && !answered { if selected { "true" } else { "false" } },
            onclick: move |_| on_pick.call(letter.clone()),
            span { class: "quiz-key", "{option.letter}" }
            span { Rich { text: option.text.clone() } }
            if answered && !option.detail.is_empty() {
                span { class: "quiz-detail", Rich { text: option.detail.clone() } }
            }
        }
    }
}

#[component]
fn EndCard(session: Session, mistakes: usize, on_restart: EventHandler<Filter>) -> Element {
    let total = session.items.len();
    let pct = 100 * session.score / total.max(1);
    let verdict = if pct >= 85 {
        "Solid. Move on to another category or to the questions you've never seen."
    } else if pct >= 60 {
        "Not bad. Reread the explanations of the missed questions, then run \"My mistakes\"."
    } else {
        "Go over the missed questions one by one in the Bank tab, then try again."
    };

    let mut by_category: BTreeMap<String, (String, usize, usize)> = BTreeMap::new();
    for (item, ok) in &session.results {
        let entry = by_category.entry(fold(&item.category)).or_insert_with(|| (item.category.clone(), 0, 0));
        entry.2 += 1;
        if *ok {
            entry.1 += 1;
        }
    }
    let missed: Vec<Item> = session.results.iter().filter(|(_, ok)| !ok).map(|(item, _)| item.clone()).collect();
    let again = if session.filter == Filter::Mistakes && mistakes == 0 { Filter::All } else { session.filter.clone() };

    rsx! {
        div { class: "quiz-card",
            div { class: "quiz-meta",
                span { "{session.filter.label()}" }
                span { "Done" }
            }
            div { class: "quiz-score", "{session.score} / {total}" }
            p { "{verdict}" }
            if by_category.len() > 1 {
                table { class: "quiz-table",
                    thead { tr { th { "Category" } th { class: "num", "Score" } } }
                    tbody {
                        for (name, right, seen) in by_category.into_values() {
                            tr { td { "{name}" } td { class: "num", "{right} / {seen}" } }
                        }
                    }
                }
            }
            if !missed.is_empty() {
                div { class: "quiz-recap",
                    for item in missed {
                        div {
                            strong { Rich { text: item.prompt.clone() } }
                            for option in item.options.iter().filter(|o| item.correct.contains(&o.letter)) {
                                div { "{option.letter}. " Rich { text: option.text.clone() } }
                            }
                        }
                    }
                }
            }
            div { class: "quiz-actions",
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| on_restart.call(again.clone()),
                    "New round"
                }
                if mistakes > 0 {
                    Button {
                        onclick: move |_| on_restart.call(Filter::Mistakes),
                        "Review my mistakes ({mistakes})"
                    }
                }
            }
        }
    }
}

#[component]
fn BankTab(
    bank: Signal<Vec<QuizQuestion>>,
    bank_resource: Resource<Option<Vec<QuizQuestion>>>,
    session: Signal<Option<Session>>,
    count: Signal<usize>,
    tab: Signal<Tab>,
) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let mut search = use_signal(String::new);
    let mut confirming_reset = use_signal(|| false);
    let mut resetting = use_signal(|| false);

    let on_reset = move |_| {
        if resetting() {
            return;
        }
        let Some(token) = auth.token() else { return };
        resetting.set(true);
        spawn(async move {
            match api::reset_quiz_progress(&token).await {
                Ok(message) => {
                    toast.success(message, ToastOptions::new());
                    bank_resource.restart();
                }
                Err(message) => toast.error(message, ToastOptions::new()),
            }
            resetting.set(false);
            confirming_reset.set(false);
        });
    };

    let questions = bank.read();
    if questions.is_empty() {
        return rsx! {
            div { class: "quiz-notice",
                "No questions loaded. "
                button { class: "quiz-link", r#type: "button", onclick: move |_| tab.set(Tab::Import), "Import questions" }
            }
        };
    }

    let multi = questions.iter().filter(|q| q.correct.len() > 1).count();
    let mastered = questions.iter().filter(|q| q.last_correct == Some(true)).count();
    let failed = questions.iter().filter(|q| q.last_correct == Some(false)).count();
    let unseen = questions.iter().filter(|q| q.last_correct.is_none()).count();
    let summary = format!(
        "{}, {multi} with several correct answers. {mastered} mastered, {failed} missed on the last try, {unseen} never seen.",
        plural(questions.len(), "question"),
    );

    let mut categories: BTreeMap<String, (String, usize, usize)> = BTreeMap::new();
    for q in questions.iter() {
        let entry = categories.entry(fold(&q.category)).or_insert_with(|| (q.category.clone(), 0, 0));
        entry.1 += 1;
        if q.last_correct == Some(true) {
            entry.2 += 1;
        }
    }

    let term = fold(search().trim());
    let matching: Vec<QuizQuestion> = questions
        .iter()
        .filter(|q| {
            if term.is_empty() {
                return true;
            }
            let mut haystack = format!("{} {}", q.prompt, q.category);
            for o in &q.options {
                haystack.push(' ');
                haystack.push_str(&o.text);
                haystack.push(' ');
                haystack.push_str(&o.detail);
            }
            fold(&haystack).contains(&term)
        })
        .cloned()
        .collect();
    drop(questions);
    let hidden = matching.len().saturating_sub(MAX_LISTED);
    let no_match = matching.is_empty();

    rsx! {
        p { class: "quiz-muted", "{summary}" }

        section { class: "quiz-block",
            h2 { "By category" }
            p { class: "quiz-hint", "\"Mastered\" counts the questions answered correctly on the last try. Click a row to start a round on that category." }
            table { class: "quiz-table quiz-categories",
                thead { tr { th { "Category" } th { class: "num", "Questions" } th { "Mastered" } } }
                tbody {
                    for (name, n, m) in categories.into_values() {
                        tr {
                            tabindex: "0",
                            onclick: {
                                let name = name.clone();
                                move |_| {
                                    session.set(start_session(&bank.read(), Filter::Category(name.clone()), count()));
                                    tab.set(Tab::Quiz);
                                }
                            },
                            onkeydown: {
                                let name = name.clone();
                                move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter {
                                        session.set(start_session(&bank.read(), Filter::Category(name.clone()), count()));
                                        tab.set(Tab::Quiz);
                                    }
                                }
                            },
                            td { "{name}" }
                            td { class: "num", "{n}" }
                            td {
                                span { class: "quiz-minibar", i { style: "width: {100 * m / n}%;" } }
                                "{m}"
                            }
                        }
                    }
                }
            }
        }

        section { class: "quiz-block",
            h2 { "All questions" }
            input {
                class: "quiz-field",
                r#type: "search",
                placeholder: "Search prompts, answers and explanations",
                "aria-label": "Search a question",
                value: "{search}",
                oninput: move |e: FormEvent| search.set(e.value()),
            }
            div { class: "quiz-list",
                for q in matching.into_iter().take(MAX_LISTED) {
                    BankRow { key: "{q.id}", question: q, bank_resource }
                }
            }
            if hidden > 0 {
                p { class: "quiz-hint", "{hidden} more results: refine the search." }
            }
            if no_match {
                p { class: "quiz-hint", "No question matches." }
            }
        }

        section { class: "quiz-block",
            if confirming_reset() {
                p { "Clear the results of every question on your account?" }
                div { class: "quiz-actions start",
                    Button {
                        variant: ButtonVariant::Destructive,
                        disabled: resetting(),
                        onclick: on_reset,
                        if resetting() { "Clearing..." } else { "Clear" }
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        disabled: resetting(),
                        onclick: move |_| confirming_reset.set(false),
                        "Cancel"
                    }
                }
            } else {
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| confirming_reset.set(true),
                    "Clear my progress"
                }
            }
        }
    }
}

#[component]
fn BankRow(question: QuizQuestion, bank_resource: Resource<Option<Vec<QuizQuestion>>>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let mut confirming_delete = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let id = question.id;

    let on_delete = move |_| {
        if deleting() {
            return;
        }
        if !confirming_delete() {
            confirming_delete.set(true);
            return;
        }
        let Some(token) = auth.token() else { return };
        deleting.set(true);
        spawn(async move {
            match api::delete_quiz_question(id, &token).await {
                Ok(message) => {
                    toast.success(message, ToastOptions::new());
                    bank_resource.restart();
                }
                Err(message) => {
                    toast.error(message, ToastOptions::new());
                    deleting.set(false);
                    confirming_delete.set(false);
                }
            }
        });
    };

    let (state_class, state_label) = match question.last_correct {
        None => ("quiz-state new", "never seen"),
        Some(true) => ("quiz-state ok", "correct"),
        Some(false) => ("quiz-state ko", "missed"),
    };

    rsx! {
        details {
            summary {
                span { class: "quiz-muted quiz-num", "#{question.num}" }
                span {
                    Rich { text: question.prompt.clone() }
                    small { "{question.category}" }
                }
                span { class: state_class, "{state_label}" }
            }
            div { class: "quiz-list-body",
                for option in question.options.clone() {
                    div { class: if question.correct.contains(&option.letter) { "quiz-answer good" } else { "quiz-answer" },
                        span { class: "quiz-key", "{option.letter}" }
                        span { Rich { text: option.text.clone() } }
                        if !option.detail.is_empty() {
                            span { class: "quiz-answer-detail", Rich { text: option.detail.clone() } }
                        }
                    }
                }
                div { class: "quiz-actions start",
                    Button {
                        variant: if confirming_delete() { ButtonVariant::Destructive } else { ButtonVariant::Outline },
                        disabled: deleting(),
                        onclick: on_delete,
                        if deleting() { "Deleting..." } else if confirming_delete() { "Confirm deletion" } else { "Delete this question" }
                    }
                    if confirming_delete() && !deleting() {
                        Button { variant: ButtonVariant::Ghost, onclick: move |_| confirming_delete.set(false), "Cancel" }
                    }
                }
            }
        }
    }
}

#[component]
fn ImportTab(bank: Signal<Vec<QuizQuestion>>, bank_resource: Resource<Option<Vec<QuizQuestion>>>, tab: Signal<Tab>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let mut text = use_signal(String::new);
    let mut report = use_signal(|| None::<ImportReport>);
    let mut status = use_signal(|| None::<(bool, String)>);
    let mut busy = use_signal(|| false);
    let mut confirming_clear = use_signal(|| false);

    let mut save = move |mode: ImportMode| {
        if busy() {
            return;
        }
        let Some(token) = auth.token() else { return };
        busy.set(true);
        spawn(async move {
            match api::import_quiz(&token, &text(), mode, false).await {
                Ok(r) => {
                    let skipped = if r.invalid > 0 { format!(" {} skipped (prompt, answers or correct answer missing).", plural(r.invalid, "block")) } else { String::new() };
                    let done = match mode {
                        ImportMode::Add => format!("{} added.", plural(r.added, "question")),
                        ImportMode::Replace => "Bank replaced.".to_string(),
                    };
                    status.set(Some((true, format!("{done}{skipped} Your bank now holds {}.", plural(r.total, "question")))));
                    report.set(None);
                    text.set(String::new());
                    toast.success(done, ToastOptions::new());
                    bank_resource.restart();
                }
                Err(message) => status.set(Some((false, message))),
            }
            busy.set(false);
        });
    };

    let mut analyze = move || {
        if busy() || text().trim().is_empty() {
            return;
        }
        let Some(token) = auth.token() else { return };
        busy.set(true);
        report.set(None);
        status.set(None);
        spawn(async move {
            let result = api::import_quiz(&token, &text(), ImportMode::Add, true).await;
            busy.set(false);
            match result {
                // Nothing to choose between on an empty bank.
                Ok(_) if bank.peek().is_empty() => save(ImportMode::Add),
                Ok(r) => report.set(Some(r)),
                Err(message) => status.set(Some((false, message))),
            }
        });
    };

    let on_file = move |e: FormEvent| {
        let Some(file) = e.files().into_iter().next() else { return };
        spawn(async move {
            match file.read_string().await {
                Ok(content) => {
                    text.set(content);
                    analyze();
                }
                Err(_) => status.set(Some((false, format!("Unable to read {}.", file.name())))),
            }
        });
    };

    let on_clear = move |_| {
        if busy() {
            return;
        }
        let Some(token) = auth.token() else { return };
        busy.set(true);
        spawn(async move {
            match api::clear_quiz_bank(&token).await {
                Ok(message) => {
                    status.set(Some((true, message)));
                    report.set(None);
                    bank_resource.restart();
                }
                Err(message) => status.set(Some((false, message))),
            }
            busy.set(false);
            confirming_clear.set(false);
        });
    };

    let bank_size = bank.read().len();

    rsx! {
        p { class: "quiz-muted",
            "Paste your quizzes or pick a text file. They are read on the server; you then choose whether to add them to your bank or replace it."
        }

        section { class: "quiz-block",
            textarea {
                class: "quiz-field quiz-textarea",
                "aria-label": "Quizzes in text format",
                placeholder: "Quiz 12\n* Category : …\n* Question : …\n* Answer A : …\n* Detail answer A : …\n* Correct answer : A",
                value: "{text}",
                oninput: move |e: FormEvent| {
                    text.set(e.value());
                    report.set(None);
                },
            }
            div { class: "quiz-actions start",
                Button {
                    disabled: busy() || text().trim().is_empty(),
                    onclick: move |_| analyze(),
                    if busy() { "Reading..." } else { "Load this text" }
                }
                label { class: "button quiz-file", "data-style": "outline",
                    "Choose a file"
                    input { r#type: "file", accept: ".txt,.md,text/plain", onchange: on_file }
                }
            }

            if let Some(r) = report() {
                div { class: "quiz-status",
                    p {
                        strong { "{plural(r.read, \"question\")} read." }
                        if r.invalid > 0 { " {plural(r.invalid, \"block\")} skipped (prompt, answers or correct answer missing)." }
                        " {plural(r.fresh, \"new one\")}, {r.read - r.fresh} already in your bank."
                    }
                    div { class: "quiz-actions start",
                        Button {
                            disabled: busy() || r.fresh == 0,
                            onclick: move |_| save(ImportMode::Add),
                            "Add to the bank (+{r.fresh})"
                        }
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: busy(),
                            onclick: move |_| save(ImportMode::Replace),
                            "Replace the bank ({bank_size} → {r.unique})"
                        }
                    }
                }
            }
            if let Some((ok, message)) = status() {
                div { class: if ok { "quiz-status ok" } else { "quiz-status ko" },
                    "{message} "
                    if ok && bank_size > 0 {
                        button { class: "quiz-link", r#type: "button", onclick: move |_| tab.set(Tab::Quiz), "Start the quiz" }
                    }
                }
            }
        }

        if bank_size > 0 {
            section { class: "quiz-block",
                if confirming_clear() {
                    p { "Delete all {plural(bank_size, \"question\")} and your progress on them?" }
                    div { class: "quiz-actions start",
                        Button { variant: ButtonVariant::Destructive, disabled: busy(), onclick: on_clear, "Empty the bank" }
                        Button { variant: ButtonVariant::Ghost, disabled: busy(), onclick: move |_| confirming_clear.set(false), "Cancel" }
                    }
                } else {
                    Button { variant: ButtonVariant::Outline, onclick: move |_| confirming_clear.set(true), "Empty the bank" }
                }
            }
        }

        section { class: "quiz-block",
            h2 { "Expected format" }
            p {
                "Each question starts with a "
                code { "Quiz N" }
                " line or is separated from the previous one by "
                code { "---" }
                ", followed by the lines "
                code { "* Category :" } ", " code { "* Question :" } ", " code { "* Answer A :" } " to "
                code { "* Answer H :" } ", " code { "* Detail answer A :" } " and " code { "* Correct answer :" } "."
            }
            ul {
                li {
                    strong { "Correct answer" }
                    ": one or more letters ("
                    code { "A" } ", " code { "A,C" } ", " code { "A and C" }
                    "), numbers 1 to 8, or the exact text of an answer."
                }
                li {
                    strong { "Shared explanation" }
                    ": "
                    code { "* Details answers B, C and D : …" }
                }
                li {
                    strong { "Category" }
                    ": the text before \"/\" becomes a quick filter (\"Rootkits / Detection\" is filed under \"Rootkits\")."
                }
                li {
                    "Bold keys ("
                    code { "- **Category :** …" }
                    ") are accepted, and lines without a key continue the previous field. In texts, "
                    code { "`code`" } " is shown in monospace and " code { "**bold**" } " in bold."
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Segment {
    Text(String),
    Bold(String),
    Code(String),
}

/// Splits `` `code` `` spans first (a `*` inside code is usually a pointer,
/// not emphasis), then `**bold**` in what's left.
fn segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        match after.find(['`', '\n']) {
            Some(end) if end > 0 && after[end..].starts_with('`') => {
                bold_segments(&rest[..start], &mut out);
                out.push(Segment::Code(after[..end].to_string()));
                rest = &after[end + 1..];
            }
            _ => {
                bold_segments(&rest[..start + 1], &mut out);
                rest = after;
            }
        }
    }
    bold_segments(rest, &mut out);
    out
}

fn bold_segments(text: &str, out: &mut Vec<Segment>) {
    let mut rest = text;
    while let Some(start) = rest.find("**") {
        let after = &rest[start + 2..];
        match after.find(['*', '\n']) {
            Some(end) if end > 0 && after[end..].starts_with("**") => {
                if start > 0 {
                    out.push(Segment::Text(rest[..start].to_string()));
                }
                out.push(Segment::Bold(after[..end].to_string()));
                rest = &after[end + 2..];
            }
            _ => {
                out.push(Segment::Text(rest[..start + 2].to_string()));
                rest = after;
            }
        }
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest.to_string()));
    }
}

/// Renders quiz text with `` `code` `` and `**bold**`, keeping line breaks
/// (via `white-space: pre-line` in the stylesheet) — no raw HTML involved.
#[component]
fn Rich(text: String) -> Element {
    rsx! {
        span { class: "quiz-rich",
            for segment in segments(&text) {
                match segment {
                    Segment::Text(t) => rsx! { "{t}" },
                    Segment::Bold(t) => rsx! { strong { "{t}" } },
                    Segment::Code(t) => rsx! { code { "{t}" } },
                }
            }
        }
    }
}
