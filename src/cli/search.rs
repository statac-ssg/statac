//! `statac search`: reads the site and searches it for some words.

use rayon::prelude::*;

use crate::ids::LanguageId;
use crate::problem::{Expected, Found, Kind, Problem, Severity};
use crate::search::{Index, Options, Results, SHOWN, code, records, results};
use crate::source::Sources;
use crate::templates::Renderer;

pub(crate) struct Outcome {
    renderer: Option<Renderer>,
    /// The errors that stopped the site being read.
    pub(crate) errors: Vec<Problem>,
    /// The language asked for, where the site declares none of its code.
    pub(crate) mistake: Option<Problem>,
    /// What each language searched found, by its code, in the site's
    /// order of languages, at most [`SHOWN`] pages shown of each.
    pub(crate) found: Vec<(String, Results)>,
}

impl Outcome {
    pub(crate) fn sources(&self) -> Option<&Sources> {
        self.renderer.as_ref().map(Renderer::sources)
    }
}

pub(crate) fn run(options: &Options) -> Outcome {
    if let Err(problem) = crate::build::a_folder(&options.site) {
        return Outcome {
            renderer: None,
            errors: vec![problem],
            mistake: None,
            found: Vec::new(),
        };
    }
    let run = crate::content::Run {
        today: options.today,
        drafts: options.drafts,
        url: None,
        renamed: Vec::new(),
    };
    let prepared = crate::build::prepared(&options.site, &run);
    let errors: Vec<Problem> = (prepared.problems.into_iter())
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .collect();
    let renderer = prepared.renderer;
    let site = renderer.site();
    let mut languages: Vec<(String, LanguageId)> = (site.languages.iter())
        .map(|(id, language)| (code(language).to_owned(), id))
        .collect();
    let mut mistake = None;
    if let Some(asked) = &options.language {
        match languages.iter().position(|(code, _)| code == asked) {
            Some(at) => languages = vec![languages.swap_remove(at)],
            None => {
                let codes = languages.iter().map(|(code, _)| code.clone()).collect();
                mistake = Some(Problem::nowhere(Kind::WrongKind {
                    field: "--language".into(),
                    expected: Expected::OneOf(codes),
                    found: Found::Written(asked.clone()),
                    respelt: None,
                }));
            }
        }
    }
    if !errors.is_empty() || mistake.is_some() {
        return Outcome {
            renderer: Some(renderer),
            errors,
            mistake,
            found: Vec::new(),
        };
    }
    let found = (languages.into_par_iter())
        .map(|(code, language)| {
            let index = Index::of(site, language, records(&renderer, language));
            let results = results(&renderer, &index, &options.words, false, SHOWN);
            (code, results)
        })
        .collect();
    Outcome {
        renderer: Some(renderer),
        errors,
        mistake,
        found,
    }
}
