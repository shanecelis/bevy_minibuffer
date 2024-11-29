enum Water {
    Fresh,
    Gray,
    Ballast
}
enum Label {
    Total,
    Water(Water)
    ...
}
pub fn update_water_tooltip(
    water_query: Query<&Water>,
    mut text_query: Query<(&mut Text, &Label),
) {
    for mut (text, label) in &mut text_query {
        let amount = match label {
            Label::Total => water_query.iter().len()
            Label::Water(water) => water_query.iter().filter(|w| w == water).len(),
        }
        text.0 = String::from(format!("{} L", amount));
    }
}
