use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use crate::rivens::inventory::raw_inventory::Auction;

    #[test]
    fn test_date_time() {
        let input = r#"{
        "starting_price": 160,
        "minimal_reputation": 0,
        "item": {
          "mastery_level": 14,
          "name": "croni-toxicta",
          "polarity": "naramon",
          "attributes": [
            {
              "value": 7.7,
              "positive": true,
              "url_name": "fire_rate_/_attack_speed"
            },
            {
              "value": 16.1,
              "positive": true,
              "url_name": "finisher_damage"
            },
            {
              "value": 12.5,
              "positive": true,
              "url_name": "toxin_damage"
            },
            {
              "value": -10.4,
              "positive": false,
              "url_name": "status_chance"
            }
          ],
          "weapon_url_name": "skana",
          "re_rolls": 0,
          "type": "riven",
          "mod_rank": 0
        },
        "buyout_price": 160,
        "note": "",
        "visible": false,
        "owner": "6457e7aa3545810677d216a5",
        "platform": "pc",
        "closed": false,
        "top_bid": null,
        "winner": null,
        "is_marked_for": null,
        "marked_operation_at": null,
        "created": "2024-05-04T19:21:58.000+00:00",
        "updated": "2024-05-04T19:21:58.000+00:00",
        "note_raw": "",
        "is_direct_sell": true,
        "id": "66368ad69454320dffff15f1",
        "private": false
      }"#;
        let auction: Auction = serde_json::from_str(input).unwrap();
        let sample = datetime!(2024-05-04 19:21:58.000+00:00);

        assert_eq!(sample, auction.updated)
    }
}
