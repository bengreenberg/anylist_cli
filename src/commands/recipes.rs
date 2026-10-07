use anylist_rs::{AnyListClient, AnyListError, Ingredient, Recipe, RecipeCollection};
use chrono::{Local, NaiveDate};
use clap::{Arg, ArgMatches, Command};

use crate::auth::read_tokens;
use crate::error::CliError;

fn display_recipe_categories(mut categories: Vec<RecipeCollection>) {
    if categories.is_empty() {
        println!("No recipe categories found.");
        return;
    }

    categories.sort_by_key(|category| category.name().to_lowercase());
    println!("\nRecipe Categories:");
    println!("{}", "=".repeat(18));
    println!();
    for category in categories {
        println!("  {} ({})", category.name(), category.id());
    }
    println!();
}

fn find_recipe_category<'a>(
    categories: &'a [RecipeCollection],
    identifier: &str,
) -> Result<&'a RecipeCollection, AnyListError> {
    if let Some(category) = categories
        .iter()
        .find(|category| category.id() == identifier)
    {
        return Ok(category);
    }

    let name = identifier.to_lowercase();
    let mut matches = categories
        .iter()
        .filter(|category| category.name().to_lowercase() == name);
    let category = matches.next().ok_or_else(|| {
        AnyListError::NotFound(format!(
            "Recipe category '{}' not found. Use 'recipe categories' to list categories.",
            identifier
        ))
    })?;
    if matches.next().is_some() {
        return Err(AnyListError::Other(format!(
            "Multiple recipe categories match '{}'. Use a category ID from 'recipe categories'.",
            identifier
        )));
    }
    Ok(category)
}

fn display_recipe_list(recipes: Vec<Recipe>) {
    if recipes.is_empty() {
        println!("No recipes found.");
        return;
    }

    println!("\nYour Recipes:");
    println!("{}", "=".repeat(13));
    println!();

    // Sort recipes by name
    let mut sorted = recipes;
    sorted.sort_by(|a, b| a.name().to_lowercase().cmp(&b.name().to_lowercase()));

    for recipe in sorted {
        let ingredient_count = recipe.ingredients().len();
        let step_count = recipe.preparation_steps().len();
        print!("  \x1B[1m{}\x1B[0m ({})", recipe.name(), recipe.id());

        if ingredient_count > 0 || step_count > 0 {
            print!(" ({} ingredients, {} steps)", ingredient_count, step_count);
        }

        if let Some(rating) = recipe.rating() {
            let stars = "★".repeat(rating as usize);
            print!(" {}", stars);
        }

        println!();
    }
    println!();
}

fn display_ingredient(ingredient: &Ingredient) {
    print!("    • {}", ingredient.name());
    if let Some(qty) = &ingredient.quantity() {
        print!(": {}", qty);
    }
    if let Some(note) = &ingredient.note() {
        print!(" ({})", note);
    }
    println!();
}

fn display_recipe_detail(
    recipe: &Recipe,
    collections: &[RecipeCollection],
    last_prepared: Option<NaiveDate>,
) {
    println!("\n\x1B[1m{}\x1B[0m", recipe.name());
    println!("{}", "=".repeat(recipe.name().len()));
    println!();

    // Display ID
    println!("ID: {}", recipe.id());

    // AnyList groups recipes into collections, which serve as recipe categories.
    let mut categories: Vec<&str> = collections
        .iter()
        .filter(|collection| collection.recipe_ids().iter().any(|id| id == recipe.id()))
        .map(|collection| collection.name())
        .collect();
    categories.sort_by_key(|name| name.to_lowercase());
    if categories.is_empty() {
        println!("Categories: None");
    } else {
        println!("Categories: {}", categories.join(", "));
    }

    // Display rating
    if let Some(rating) = recipe.rating() {
        let stars = "★".repeat(rating as usize);
        println!("Rating: {}", stars);
    }

    // Display source
    if let Some(source_name) = &recipe.source_name() {
        print!("Source: {}", source_name);
        if let Some(source_url) = &recipe.source_url() {
            print!(" ({})", source_url);
        }
        println!();
    }

    // Display servings
    if let Some(servings) = &recipe.servings() {
        println!("Servings: {}", servings);
    }

    // Display times (convert from seconds to minutes)
    if let Some(prep_time) = recipe.prep_time() {
        println!("Prep Time: {} minutes", prep_time / 60);
    } else {
        println!("Prep Time: Not specified");
    }
    if let Some(cook_time) = recipe.cook_time() {
        println!("Cook Time: {} minutes", cook_time / 60);
    }

    if let Some(date) = last_prepared {
        println!("Last Prepared: {} (meal plan)", date);
    } else {
        println!("Last Prepared: Not recorded in meal plan");
    }

    // Display note
    if let Some(note) = &recipe.note() {
        println!("\nNote: {}", note);
    }

    // Display ingredients
    if !recipe.ingredients().is_empty() {
        println!("\n\x1B[1mIngredients:\x1B[0m");
        for ingredient in recipe.ingredients().to_owned().into_iter() {
            display_ingredient(&ingredient);
        }
    }

    // Display preparation steps
    if !recipe.preparation_steps().is_empty() {
        println!("\n\x1B[1mPreparation:\x1B[0m");
        for (i, step) in recipe.preparation_steps().iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
    }

    println!();
}

pub fn command() -> Command {
    Command::new("recipe")
        .about("View and manage your AnyList recipes")
        .long_about(
            "View and manage your AnyList recipes.\n\n\
             By default, this command shows all your recipes.\n\
             Use subcommands to view recipe details.",
        )
        .subcommand(
            Command::new("list")
                .about("List recipes, optionally filtered by category")
                .long_about(
                    "Display recipes with IDs, ingredient counts, and step counts. Optionally filter \
                     by an AnyList recipe collection using its ID or case-insensitive name.",
                )
                .arg(
                    Arg::new("category")
                        .long("category")
                        .help("Filter by category ID or case-insensitive name")
                        .value_name("CATEGORY_NAME_OR_ID"),
                ),
        )
        .subcommand(
            Command::new("categories")
                .about("List recipe categories with their IDs")
                .long_about("List AnyList recipe collections as category names and IDs"),
        )
        .subcommand(
            Command::new("get")
                .about("Display details for a specific recipe")
                .long_about(
                    "Display recipe details, including preparation time, categories (AnyList \
                     collections), last prepared date (from the meal plan), ingredients, and \
                     preparation steps.",
                )
                .arg(
                    Arg::new("name")
                        .help("Name or ID of the recipe to display")
                        .required(true)
                        .value_name("RECIPE_NAME_OR_ID"),
                ),
        )
}

pub async fn exec_command(matches: &ArgMatches) -> Result<(), CliError> {
    let tokens = read_tokens()?;
    let client = AnyListClient::from_tokens(tokens)?;

    match matches.subcommand() {
        Some(("categories", _)) => {
            let categories = client.get_recipe_collections().await?;
            display_recipe_categories(categories);
        }
        Some(("list", sub_matches)) => {
            let mut recipes = client.get_recipes().await?;
            if let Some(identifier) = sub_matches.get_one::<String>("category") {
                let categories = client.get_recipe_collections().await?;
                let category = find_recipe_category(&categories, identifier)?;
                recipes.retain(|recipe| category.recipe_ids().iter().any(|id| id == recipe.id()));
            }
            display_recipe_list(recipes);
        }
        Some(("get", sub_matches)) => {
            let identifier = sub_matches
                .get_one::<String>("name")
                .expect("required argument");

            // Try to get by name first, fall back to ID
            let recipe = match client.get_recipe_by_name(identifier).await {
                Ok(r) => r,
                Err(_) => client.get_recipe_by_id(identifier).await?,
            };

            let collections = client.get_recipe_collections().await?;
            let today = Local::now().date_naive();
            // Search the full meal-plan history through today, excluding future plans.
            let events = client
                .get_meal_plan_events("0001-01-01", &today.to_string())
                .await?;
            let last_prepared = events
                .iter()
                .filter(|event| event.recipe_id() == Some(recipe.id()))
                .filter_map(|event| NaiveDate::parse_from_str(event.date(), "%Y-%m-%d").ok())
                .max();
            display_recipe_detail(&recipe, &collections, last_prepared);
        }
        _ => {
            // Default: show all recipes
            let recipes = client.get_recipes().await?;
            display_recipe_list(recipes);
        }
    }

    Ok(())
}
