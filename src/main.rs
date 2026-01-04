use rand::{seq::SliceRandom, thread_rng, Rng};
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

const STARTING_CHIPS: i32 = 1000;
const SMALL_BLIND: i32 = 10;
const BIG_BLIND: i32 = 20;
const MIN_RAISE: i32 = 20;
const MIN_BET_AMOUNT: i32 = 30;
const MAX_BET_AMOUNT: i32 = 150;
const BOT_THINK_TIME_MS: u64 = 800;
const PHASE_TRANSITION_TIME_MS: u64 = 600;
const DEBUG_MODE: bool = false;

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if DEBUG_MODE {
            println!($($arg)*);
        }
    };
}

slint::include_modules!();

#[derive(Clone, Debug)]
struct Card {
    rank: String,
    suit: String,
    value: i32,
}

impl Card {
    fn new(rank: &str, suit: &str, value: i32) -> Self {
        Self {
            rank: rank.to_string(),
            suit: suit.to_string(),
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HandRank {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct EvaluatedHand {
    rank: HandRank,
    primary_value: i32,
    secondary_values: Vec<i32>,
    kickers: Vec<i32>,
}

fn evaluate_hand(hole_cards: &[Card], community_cards: &[Card]) -> EvaluatedHand {
    let mut all_cards: Vec<(i32, &str)> = hole_cards
        .iter()
        .chain(community_cards.iter())
        .map(|c| (c.value, c.suit.as_str()))
        .collect();

    all_cards.sort_by_key(|a| a.0);
    all_cards.dedup_by_key(|a| a.0);

    let values: Vec<i32> = all_cards.iter().map(|a| a.0).collect();
    let suits: Vec<&str> = all_cards.iter().map(|a| a.1).collect();

    let suit_counts: std::collections::HashMap<&str, usize> =
        suits
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, &suit| {
                *acc.entry(suit).or_insert(0) += 1;
                acc
            });
    let max_suit_count = suit_counts.values().max().copied().unwrap_or(0);
    let _flush_suit = if max_suit_count >= 5 {
        suit_counts
            .iter()
            .find(|(_, &v)| v == max_suit_count)
            .map(|(&s, _)| s)
    } else {
        None
    };

    let is_flush = max_suit_count >= 5;

    let mut is_straight = false;
    let straight_high = if values.len() >= 5 {
        for i in 0..=values.len() - 5 {
            let mut straight_values = values[i..i + 5].to_vec();
            straight_values.sort_unstable();
            let mut consecutive = true;
            for j in 0..4 {
                if straight_values[j + 1] - straight_values[j] != 1 {
                    consecutive = false;
                    break;
                }
            }
            if consecutive {
                is_straight = true;
                break;
            }
        }
        if !is_straight && values.len() >= 5 {
            let lowest = values[0];
            let highest = values[values.len() - 1];
            if highest - lowest == 12 {
                let has_ace = values.contains(&14);
                let has_two = values.contains(&2);
                if has_ace && has_two {
                    let wheel = [2, 3, 4, 5, 14];
                    let mut found_wheel = true;
                    for v in &wheel {
                        if !values.contains(v) {
                            found_wheel = false;
                            break;
                        }
                    }
                    if found_wheel {
                        is_straight = true;
                    }
                }
            }
        }
        values.iter().max().copied().unwrap_or(0)
    } else {
        0
    };

    let value_counts: std::collections::HashMap<i32, usize> =
        values
            .iter()
            .fold(std::collections::HashMap::new(), |mut acc, &val| {
                *acc.entry(val).or_insert(0) += 1;
                acc
            });

    let four_of_kind: Vec<_> = value_counts
        .iter()
        .filter(|(_, &c)| c == 4)
        .map(|(&v, _)| v)
        .collect();
    let three_of_kind: Vec<_> = value_counts
        .iter()
        .filter(|(_, &c)| c == 3)
        .map(|(&v, _)| v)
        .collect();
    let pairs: Vec<_> = value_counts
        .iter()
        .filter(|(_, &c)| c == 2)
        .map(|(&v, _)| v)
        .collect();

    let has_full_house = !three_of_kind.is_empty() && !pairs.is_empty();
    let has_three_of_kind = !three_of_kind.is_empty();
    let has_two_pair = pairs.len() >= 2;

    if is_flush && is_straight {
        EvaluatedHand {
            rank: HandRank::StraightFlush,
            primary_value: straight_high,
            secondary_values: Vec::new(),
            kickers: Vec::new(),
        }
    } else if !four_of_kind.is_empty() {
        let four_val = four_of_kind[0];
        let kicker = values
            .iter()
            .filter(|&&v| v != four_val)
            .max()
            .copied()
            .unwrap_or(0);
        EvaluatedHand {
            rank: HandRank::FourOfAKind,
            primary_value: four_val,
            secondary_values: vec![kicker],
            kickers: Vec::new(),
        }
    } else if has_full_house {
        let three_val = three_of_kind[0];
        let pair_val = pairs[0];
        EvaluatedHand {
            rank: HandRank::FullHouse,
            primary_value: three_val,
            secondary_values: vec![pair_val],
            kickers: Vec::new(),
        }
    } else if is_flush {
        let sorted_flush: Vec<i32> = values.iter().copied().take(5).collect();
        let kickers: Vec<i32> = values
            .iter()
            .filter(|&&v| !sorted_flush.contains(&v))
            .copied()
            .take(2)
            .collect();
        EvaluatedHand {
            rank: HandRank::Flush,
            primary_value: sorted_flush.iter().max().copied().unwrap_or(0),
            secondary_values: sorted_flush.iter().skip(1).copied().collect(),
            kickers,
        }
    } else if is_straight {
        EvaluatedHand {
            rank: HandRank::Straight,
            primary_value: straight_high,
            secondary_values: Vec::new(),
            kickers: Vec::new(),
        }
    } else if has_three_of_kind {
        let three_val = three_of_kind[0];
        let kickers: Vec<i32> = values
            .iter()
            .filter(|&&v| v != three_val)
            .copied()
            .take(2)
            .collect();
        EvaluatedHand {
            rank: HandRank::ThreeOfAKind,
            primary_value: three_val,
            secondary_values: kickers,
            kickers: Vec::new(),
        }
    } else if has_two_pair {
        let mut sorted_pairs: Vec<i32> = pairs.clone();
        sorted_pairs.sort_unstable();
        sorted_pairs.reverse();
        let high_pair = sorted_pairs[0];
        let low_pair = sorted_pairs[1];
        let kicker = values
            .iter()
            .filter(|&&v| !pairs.contains(&v))
            .max()
            .copied()
            .unwrap_or(0);
        EvaluatedHand {
            rank: HandRank::TwoPair,
            primary_value: high_pair,
            secondary_values: vec![low_pair, kicker],
            kickers: Vec::new(),
        }
    } else if pairs.len() == 1 {
        let pair_val = pairs[0];
        let kickers: Vec<i32> = values
            .iter()
            .filter(|&&v| v != pair_val)
            .copied()
            .take(3)
            .collect();
        EvaluatedHand {
            rank: HandRank::Pair,
            primary_value: pair_val,
            secondary_values: kickers.clone(),
            kickers: Vec::new(),
        }
    } else {
        let top_five: Vec<i32> = values.iter().copied().take(5).collect();
        EvaluatedHand {
            rank: HandRank::HighCard,
            primary_value: top_five.iter().max().copied().unwrap_or(0),
            secondary_values: top_five.iter().skip(1).copied().collect(),
            kickers: Vec::new(),
        }
    }
}

fn compare_hands(hand1: &EvaluatedHand, hand2: &EvaluatedHand) -> i32 {
    if hand1.rank != hand2.rank {
        return hand1.rank as i32 - hand2.rank as i32;
    }
    if hand1.primary_value != hand2.primary_value {
        return hand1.primary_value - hand2.primary_value;
    }
    for (v1, v2) in hand1
        .secondary_values
        .iter()
        .zip(hand2.secondary_values.iter())
    {
        if v1 != v2 {
            return v1 - v2;
        }
    }
    0
}

#[derive(Clone, Debug)]
struct Player {
    name: String,
    chips: i32,
    bet: i32,
    cards: Vec<Card>,
    is_user: bool,
    last_action: String,
}

impl Player {
    fn new(name: &str, is_user: bool) -> Self {
        Self {
            name: name.to_string(),
            chips: STARTING_CHIPS,
            bet: 0,
            cards: Vec::new(),
            is_user,
            last_action: String::new(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum GamePhase {
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
}

struct PokerGame {
    deck: Vec<Card>,
    community_cards: Vec<Card>,
    players: Vec<Player>,
    current_player: usize,
    phase: GamePhase,
    pot: i32,
    current_bet: i32,
    dealer_position: usize,
    small_blind: i32,
    big_blind: i32,
    hand_complete: bool,
    showdown_done: bool,
    game_over: bool,
}

impl PokerGame {
    fn new() -> Self {
        let players = vec![Player::new("You", true), Player::new("Bot", false)];

        Self {
            deck: Vec::new(),
            community_cards: Vec::new(),
            players,
            current_player: 0,
            phase: GamePhase::PreFlop,
            pot: 0,
            current_bet: 0,
            dealer_position: 0,
            small_blind: SMALL_BLIND,
            big_blind: BIG_BLIND,
            hand_complete: false,
            showdown_done: false,
            game_over: false,
        }
    }

    fn create_deck(&mut self) {
        self.deck.clear();
        let ranks = [
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A",
        ];
        let suits = ["♠", "♥", "♦", "♣"];
        let mut value = 2;
        for rank in &ranks {
            for suit in &suits {
                self.deck.push(Card::new(rank, suit, value));
            }
            value += 1;
        }
    }

    fn shuffle_deck(&mut self) {
        let mut rng = thread_rng();
        self.deck.shuffle(&mut rng);
    }

    fn deal_card(&mut self) -> Option<Card> {
        self.deck.pop()
    }

    fn start_hand(&mut self) {
        debug_log!("\n=== STARTING NEW HAND ===");

        self.create_deck();
        self.shuffle_deck();
        self.community_cards.clear();
        self.pot = 0;
        self.current_bet = 0;
        self.phase = GamePhase::PreFlop;
        self.hand_complete = false;
        self.showdown_done = false;
        self.game_over = false;

        for player in &mut self.players {
            player.bet = 0;
            player.cards.clear();
            player.last_action = String::new();
        }

        debug_log!(
            "You: ${}  |  Bot: ${}",
            self.players[0].chips,
            self.players[1].chips
        );

        let dealer_idx = self.dealer_position;
        let sb_idx = (self.dealer_position + 1) % self.players.len();
        let bb_idx = (self.dealer_position + 2) % self.players.len();

        let (dealer_name, sb_name, bb_name) = if self.players.len() == 2 {
            if bb_idx == dealer_idx {
                (
                    self.players[dealer_idx].name.clone(),
                    self.players[sb_idx].name.clone(),
                    self.players[dealer_idx].name.clone(),
                )
            } else {
                (
                    self.players[dealer_idx].name.clone(),
                    self.players[sb_idx].name.clone(),
                    self.players[bb_idx].name.clone(),
                )
            }
        } else {
            (
                self.players[dealer_idx].name.clone(),
                self.players[sb_idx].name.clone(),
                self.players[bb_idx].name.clone(),
            )
        };

        debug_log!(
            "Dealer: {}  |  SB: {}  |  BB: {}",
            dealer_name,
            sb_name,
            bb_name
        );

        self.post_blinds();
        self.deal_hole_cards();

        self.current_player = (self.dealer_position + 3) % self.players.len();
        debug_log!(
            "\n>>> {}'s turn ({})",
            self.players[self.current_player].name,
            self.get_phase_name()
        );
        debug_log!("Pot: ${}  |  Current bet: ${}", self.pot, self.current_bet);
    }

    fn post_blinds(&mut self) {
        let sb_player = (self.dealer_position + 1) % self.players.len();
        let bb_player = (self.dealer_position + 2) % self.players.len();

        self.players[sb_player].bet = self.small_blind;
        self.players[sb_player].chips -= self.small_blind;
        self.players[sb_player].last_action = format!("SB: ${}", self.small_blind);
        debug_log!(
            "  {} posts small blind: ${}",
            self.players[sb_player].name,
            self.small_blind
        );

        self.players[bb_player].bet = self.big_blind;
        self.players[bb_player].chips -= self.big_blind;
        self.players[bb_player].last_action = format!("BB: ${}", self.big_blind);
        debug_log!(
            "  {} posts big blind: ${}",
            self.players[bb_player].name,
            self.big_blind
        );

        self.current_bet = self.big_blind;
        self.pot += self.small_blind + self.big_blind;
    }

    fn deal_hole_cards(&mut self) {
        debug_log!("\n Dealing hole cards...");
        for i in 0..self.players.len() {
            if let Some(card) = self.deal_card() {
                self.players[i].cards.push(card.clone());
            }
            if let Some(card) = self.deal_card() {
                self.players[i].cards.push(card.clone());
            }
            if self.players[i].is_user {
                debug_log!(
                    "  Your cards: {} {} | {} {}",
                    self.players[i].cards[0].rank,
                    self.players[i].cards[0].suit,
                    self.players[i].cards[1].rank,
                    self.players[i].cards[1].suit
                );
            } else {
                debug_log!("  Bot cards: [hidden] [hidden]");
            }
        }
    }

    fn deal_community_cards(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(card) = self.deal_card() {
                self.community_cards.push(card);
            }
        }
    }

    fn next_phase(&mut self) {
        match self.phase {
            GamePhase::PreFlop => {
                debug_log!("\n=== THE FLOP ===");
                self.deal_community_cards(3);
                self.phase = GamePhase::Flop;
            }
            GamePhase::Flop => {
                debug_log!("\n=== THE TURN ===");
                self.deal_community_cards(1);
                self.phase = GamePhase::Turn;
            }
            GamePhase::Turn => {
                debug_log!("\n=== THE RIVER ===");
                self.deal_community_cards(1);
                self.phase = GamePhase::River;
            }
            GamePhase::River => {
                debug_log!("\n=== SHOWDOWN ===");
                self.phase = GamePhase::Showdown;
                self.do_showdown();
                return;
            }
            GamePhase::Showdown => {}
        }
        self.finish_phase_transition();
    }

    fn finish_phase_transition(&mut self) {
        self.current_bet = 0;
        for player in &mut self.players {
            player.bet = 0;
        }
        self.current_player = (self.dealer_position + 1) % self.players.len();

        let community_str: String = self
            .community_cards
            .iter()
            .map(|c| format!("{} {}", c.rank, c.suit))
            .collect::<Vec<_>>()
            .join(" | ");
        debug_log!("\nCommunity cards: {}", community_str);
        debug_log!("\n>>> {}'s turn", self.players[self.current_player].name);
        debug_log!("Pot: ${}  |  Current bet: $0", self.pot);
    }

    fn get_phase_name(&self) -> String {
        match self.phase {
            GamePhase::PreFlop => "Pre-Flop".to_string(),
            GamePhase::Flop => "Flop".to_string(),
            GamePhase::Turn => "Turn".to_string(),
            GamePhase::River => "River".to_string(),
            GamePhase::Showdown => "Showdown!".to_string(),
        }
    }

    fn get_next_player(&self) -> usize {
        (self.current_player + 1) % self.players.len()
    }

    fn all_players_matched(&self) -> bool {
        self.players
            .iter()
            .all(|p| p.bet == self.current_bet || p.cards.is_empty())
    }

    fn move_to_next_player(&mut self) {
        self.current_player = self.get_next_player();
    }

    fn player_action(&mut self, action: &str, amount: Option<i32>) -> bool {
        let player = &mut self.players[self.current_player];
        let bet_amount = amount.unwrap_or(0);

        match action {
            "fold" => {
                debug_log!("  {} FOLDS!", player.name);
                player.cards.clear();
                player.last_action = "Folded".to_string();
                self.move_to_next_player();
                return true;
            }
            "check" => {
                if player.bet >= self.current_bet {
                    debug_log!("  {} CHECKS", player.name);
                    player.last_action = "Check".to_string();
                    self.move_to_next_player();
                    return true;
                }
            }
            "bet" | "raise" => {
                let to_bet = bet_amount.max(self.current_bet + MIN_RAISE);
                if player.chips >= to_bet {
                    let call_part = (self.current_bet - player.bet).max(0);
                    let actual_bet = to_bet - call_part;
                    player.chips -= call_part;
                    player.chips -= actual_bet;
                    player.bet = to_bet;
                    let action_type = if action == "bet" { "BETS" } else { "RAISES" };
                    debug_log!("  {} {} ${}", player.name, action_type, actual_bet);
                    player.last_action = format!("${}", to_bet);
                    self.current_bet = to_bet;
                    self.pot += to_bet;
                    self.move_to_next_player();
                    return true;
                }
            }
            "call" => {
                let call_amount = self.current_bet - player.bet;
                if player.chips >= call_amount {
                    player.chips -= call_amount;
                    player.bet = self.current_bet;
                    debug_log!("  {} CALLS ${}", player.name, call_amount);
                    player.last_action = format!("Call: ${}", call_amount);
                    self.pot += call_amount;
                    self.move_to_next_player();
                    return true;
                }
            }
            "all-in" => {
                let all_in = player.chips;
                if all_in > 0 {
                    player.chips = 0;
                    player.bet += all_in;
                    debug_log!("  {} GOES ALL-IN FOR ${}!", player.name, all_in);
                    player.last_action = format!("All-In: ${}", all_in);
                    self.pot += all_in;
                    if player.bet > self.current_bet {
                        self.current_bet = player.bet;
                    }
                    self.move_to_next_player();
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn make_bot_move(&mut self) {
        if self.hand_complete || self.phase == GamePhase::Showdown {
            return;
        }

        let player_chips = self.players[self.current_player].chips;
        let call_amount = (self.current_bet - self.players[self.current_player].bet).max(0);
        let to_call = call_amount;

        let bot_hand = evaluate_hand(
            &self.players[self.current_player].cards,
            &self.community_cards,
        );
        let hand_strength = bot_hand.rank as i32 * 100 + bot_hand.primary_value;

        let mut rng = thread_rng();

        let actions = match self.phase {
            GamePhase::PreFlop => {
                if to_call == 0 {
                    vec!["check", "bet", "raise", "fold"]
                } else {
                    vec!["call", "raise", "fold"]
                }
            }
            GamePhase::Flop => {
                if to_call == 0 {
                    vec!["check", "bet", "fold"]
                } else {
                    vec!["call", "raise", "fold"]
                }
            }
            GamePhase::Turn => {
                if to_call == 0 {
                    vec!["check", "bet", "fold"]
                } else {
                    vec!["call", "raise", "fold"]
                }
            }
            GamePhase::River => {
                if to_call == 0 {
                    vec!["check", "bet", "fold"]
                } else {
                    vec!["call", "raise", "fold"]
                }
            }
            GamePhase::Showdown => vec![],
        };

        if actions.is_empty() {
            return;
        }

        let action = if hand_strength >= 700 {
            match self.phase {
                GamePhase::PreFlop if to_call == 0 => {
                    if rng.gen_range(0..100) < 70 {
                        "raise"
                    } else {
                        "check"
                    }
                }
                GamePhase::PreFlop => {
                    if rng.gen_range(0..100) < 80 {
                        "call"
                    } else {
                        "raise"
                    }
                }
                _ => {
                    if rng.gen_range(0..100) < 75 {
                        "raise"
                    } else {
                        "call"
                    }
                }
            }
        } else if hand_strength >= 500 {
            match self.phase {
                GamePhase::PreFlop if to_call == 0 => {
                    if rng.gen_range(0..100) < 50 {
                        "check"
                    } else {
                        "bet"
                    }
                }
                GamePhase::PreFlop => {
                    if rng.gen_range(0..100) < 60 {
                        "call"
                    } else {
                        "raise"
                    }
                }
                _ => {
                    if rng.gen_range(0..100) < 50 {
                        "call"
                    } else {
                        "raise"
                    }
                }
            }
        } else if hand_strength >= 300 {
            match self.phase {
                GamePhase::PreFlop if to_call == 0 => {
                    if rng.gen_range(0..100) < 40 {
                        "check"
                    } else {
                        "bet"
                    }
                }
                GamePhase::PreFlop => {
                    if rng.gen_range(0..100) < 40 {
                        "call"
                    } else {
                        "raise"
                    }
                }
                _ => {
                    if rng.gen_range(0..100) < 30 {
                        "call"
                    } else {
                        "raise"
                    }
                }
            }
        } else {
            match self.phase {
                GamePhase::PreFlop if to_call == 0 => {
                    if rng.gen_range(0..100) < 30 {
                        "check"
                    } else {
                        "fold"
                    }
                }
                GamePhase::PreFlop => {
                    if rng.gen_range(0..100) < 30 {
                        "call"
                    } else {
                        "fold"
                    }
                }
                _ => {
                    if rng.gen_range(0..100) < 20 {
                        "call"
                    } else {
                        "fold"
                    }
                }
            }
        };

        let bet_amount = match action {
            "bet" | "raise" => {
                let base_amount = if hand_strength >= 700 {
                    player_chips.min(MAX_BET_AMOUNT + 50)
                } else if hand_strength >= 500 {
                    player_chips.min(MAX_BET_AMOUNT)
                } else {
                    player_chips.min(MIN_BET_AMOUNT + 20)
                };
                rng.gen_range(MIN_BET_AMOUNT..=base_amount)
            }
            _ => 0,
        };

        self.player_action(action, Some(bet_amount));
    }

    fn check_phase_complete(&mut self) {
        if self.all_players_matched() {
            thread::sleep(Duration::from_millis(PHASE_TRANSITION_TIME_MS));
            self.next_phase();
        }
    }

    fn do_showdown(&mut self) {
        if self.showdown_done {
            return;
        }
        self.showdown_done = true;

        debug_log!("\n=== SHOWDOWN RESULTS ===");

        let user = &self.players[0];
        let bot = &self.players[1];

        if user.cards.len() >= 2 {
            debug_log!(
                "\n Your hand: {} {} | {} {}",
                user.cards[0].rank,
                user.cards[0].suit,
                user.cards[1].rank,
                user.cards[1].suit
            );
        } else {
            debug_log!("\n Your hand: (folded)");
        }

        if !bot.cards.is_empty() && bot.cards.len() >= 2 {
            debug_log!(
                " Bot hand: {} {} | {} {}",
                bot.cards[0].rank,
                bot.cards[0].suit,
                bot.cards[1].rank,
                bot.cards[1].suit
            );
        } else if bot.cards.is_empty() {
            debug_log!(" Bot folded!");
        } else {
            debug_log!(" Bot hand: (incomplete)");
        }

        let active_players: Vec<(usize, &Player)> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.cards.is_empty())
            .collect();

        if active_players.len() == 1 {
            let winner_idx = active_players[0].0;
            debug_log!(
                "\n  {} WINS ${} BY DEFAULT!",
                active_players[0].1.name,
                self.pot
            );
            self.players[winner_idx].chips += self.pot;
            self.game_over = true;
        } else if active_players.len() == 2 {
            let user_eval = evaluate_hand(&user.cards, &self.community_cards);
            let bot_eval = evaluate_hand(&bot.cards, &self.community_cards);

            debug_log!("\n  Your hand: {:?}", user_eval.rank);
            debug_log!("  Bot hand: {:?}", bot_eval.rank);

            let comparison = compare_hands(&user_eval, &bot_eval);

            if comparison > 0 {
                debug_log!("\n  YOU WIN ${}!", self.pot);
                self.players[0].chips += self.pot;
            } else if comparison < 0 {
                debug_log!("\n  BOT WINS ${}!", self.pot);
                self.players[1].chips += self.pot;
            } else {
                debug_log!("\n  SPLIT POT! Each gets ${}", self.pot / 2);
                self.players[0].chips += self.pot / 2;
                self.players[1].chips += self.pot / 2;
            }
        }

        self.hand_complete = true;

        debug_log!(
            "\nYour chips: ${}  |  Bot chips: ${}",
            self.players[0].chips,
            self.players[1].chips
        );
    }

    fn is_user_turn(&self) -> bool {
        self.players[self.current_player].is_user
            && !self.hand_complete
            && self.phase != GamePhase::Showdown
    }

    fn is_bot_turn(&self) -> bool {
        !self.players[self.current_player].is_user
            && !self.hand_complete
            && self.phase != GamePhase::Showdown
    }

    fn get_winner_name(&self) -> String {
        if self.players[0].chips > self.players[1].chips {
            "YOU WIN!".to_string()
        } else if self.players[1].chips > self.players[0].chips {
            "BOT WINS!".to_string()
        } else {
            "TIE GAME!".to_string()
        }
    }

    fn is_game_over(&self) -> bool {
        self.game_over || self.players.iter().any(|p| p.chips <= 0)
    }
}

fn create_card_ui_data(card: &Card) -> CardUI {
    CardUI {
        rank: card.rank.clone().into(),
        suit: card.suit.clone().into(),
        card_color: if card.suit == "♥" || card.suit == "♦" {
            "red".into()
        } else {
            "black".into()
        },
    }
}

struct AppState {
    game: Rc<RefCell<PokerGame>>,
    main_window: slint::Weak<MainWindow>,
}

impl AppState {
    fn new(window: slint::Weak<MainWindow>) -> Self {
        let game = Rc::new(RefCell::new(PokerGame::new()));
        let state = Self {
            game,
            main_window: window,
        };
        state
    }

    fn update_ui(&self) -> bool {
        let game = self.game.borrow();
        let Some(window) = self.main_window.upgrade() else {
            return false;
        };

        window.set_pot(game.pot);
        window.set_current_bet(game.current_bet);
        window.set_phase_name(game.get_phase_name().into());
        window.set_current_player_name(game.players[game.current_player].name.clone().into());
        window.set_hand_complete(game.hand_complete);

        let player_cards: Vec<CardUI> = game.players[0]
            .cards
            .iter()
            .map(create_card_ui_data)
            .collect();
        window.set_player_cards(Rc::new(VecModel::from(player_cards)).into());

        let bot_cards: Vec<CardUI> = if game.phase == GamePhase::Showdown || game.hand_complete {
            game.players[1]
                .cards
                .iter()
                .map(create_card_ui_data)
                .collect()
        } else {
            vec![
                CardUI {
                    rank: "".into(),
                    suit: "🂠".into(),
                    card_color: "gray".into()
                };
                2
            ]
        };
        window.set_bot_cards(Rc::new(VecModel::from(bot_cards)).into());

        let community_cards: Vec<CardUI> = game
            .community_cards
            .iter()
            .map(create_card_ui_data)
            .collect();
        window.set_community_cards(Rc::new(VecModel::from(community_cards)).into());

        window.set_player_chips(game.players[0].chips);
        window.set_player_bet(game.players[0].bet);
        window.set_player_last_action(game.players[0].last_action.clone().into());

        window.set_bot_chips(game.players[1].chips);
        window.set_bot_bet(game.players[1].bet);
        window.set_bot_last_action(game.players[1].last_action.clone().into());

        let is_user_turn = game.is_user_turn();
        let call_amount = game.current_bet - game.players[0].bet;
        let can_check = call_amount <= 0;
        let can_call = game.players[0].chips >= call_amount.max(0);
        let min_raise = game.current_bet + MIN_RAISE;

        window.set_show_actions(is_user_turn);
        window.set_can_check(can_check);
        window.set_can_call(can_call);
        window.set_can_fold(true);
        window.set_can_raise(game.players[0].chips >= min_raise);
        window.set_min_raise_amount(min_raise);

        window.set_show_winner(false);
        window.set_game_over(game.is_game_over());
        true
    }

    fn process_bot_turn(&self) {
        let game = self.game.borrow();
        if !game.is_bot_turn() {
            return;
        }
        drop(game);
        thread::sleep(Duration::from_millis(BOT_THINK_TIME_MS));
        loop {
            let mut game = self.game.borrow_mut();
            if !game.is_bot_turn() {
                break;
            }
            game.make_bot_move();
            game.check_phase_complete();
            let done = game.hand_complete;
            drop(game);
            self.update_ui();
            if done {
                break;
            }
            thread::sleep(Duration::from_millis(PHASE_TRANSITION_TIME_MS));
        }
    }

    fn process_action(&self, action: &str, amount: Option<i32>) {
        let mut game = self.game.borrow_mut();
        if game.player_action(action, amount) {
            debug_log!("Pot: ${}", game.pot);
            game.check_phase_complete();
            let needs_bot = game.is_bot_turn();
            drop(game);
            self.update_ui();
            if needs_bot {
                self.process_bot_turn();
            }
        } else {
            let current_player = game.current_player;
            let player_chips = game.players[current_player].chips;
            let current_bet = game.current_bet;
            let player_bet = game.players[current_player].bet;

            match action {
                "check" => {
                    game.players[current_player].last_action = "Cannot check".to_string();
                }
                "call" => {
                    let call_amount = current_bet - player_bet;
                    if player_chips < call_amount {
                        game.players[current_player].last_action =
                            "Not enough chips to call".to_string();
                    }
                }
                "bet" | "raise" => {
                    let to_bet = amount.unwrap_or(0).max(current_bet + MIN_RAISE);
                    if player_chips < to_bet {
                        game.players[current_player].last_action =
                            "Not enough chips to raise".to_string();
                    }
                }
                _ => {}
            }
            drop(game);
            self.update_ui();
        }
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            game: self.game.clone(),
            main_window: self.main_window.clone(),
        }
    }
}

fn main() {
    debug_log!("TEXAS HOLD'EM POKER vs BOT");

    let main_window = match MainWindow::new() {
        Ok(window) => window,
        Err(e) => {
            if DEBUG_MODE {
                eprintln!("Failed to create window: {}", e);
            }
            return;
        }
    };
    let weak_window = main_window.as_weak();

    let state = Rc::new(AppState::new(weak_window.clone()));

    {
        let mut game = state.game.borrow_mut();
        game.start_hand();
    }
    state.update_ui();

    debug_log!("\nClick NEW HAND to start playing!");

    let state_check = state.clone();
    main_window.on_check(move || {
        debug_log!("\n>>> You CHECK");
        state_check.process_action("check", None);
    });

    let state_call = state.clone();
    main_window.on_call(move || {
        debug_log!("\n>>> You CALL");
        state_call.process_action("call", None);
    });

    let state_fold = state.clone();
    main_window.on_fold(move || {
        debug_log!("\n>>> You FOLD");
        state_fold.process_action("fold", None);
    });

    let state_raise = state.clone();
    main_window.on_raise(move || {
        let amount = {
            let game = state_raise.game.borrow();
            game.current_bet + MIN_RAISE
        };
        debug_log!("\n>>> You RAISE to ${}", amount);
        state_raise.process_action("raise", Some(amount));
    });

    let state_all_in = state.clone();
    main_window.on_all_in(move || {
        debug_log!("\n>>> You GO ALL-IN!");
        state_all_in.process_action("all-in", None);
    });

    let state_new = state.clone();
    main_window.on_new_hand(move || {
        debug_log!("\n=== NEW HAND ===");
        let show_winner: Option<(String, bool)> = {
            let mut game = state_new.game.borrow_mut();
            if game.is_game_over() {
                debug_log!("\n=== GAME OVER ===");
                let winner = game.get_winner_name();
                let was_game_over = game.game_over;
                debug_log!("{}", winner);

                if was_game_over {
                    game.players[0].chips = STARTING_CHIPS;
                    game.players[1].chips = STARTING_CHIPS;
                    game.dealer_position = 0;
                    game.game_over = false;
                }

                drop(game);
                let window = state_new.main_window.upgrade();
                if let Some(win) = window {
                    win.set_show_winner(true);
                    win.set_winner_name(winner.clone().into());
                    win.set_hand_complete(true);
                }
                return;
            }
            game.dealer_position = (game.dealer_position + 1) % 2;
            game.start_hand();
            None
        };
        if show_winner.is_none() {
            state_new.update_ui();
        }
    });

    main_window.run().unwrap_or_else(|e| {
        if DEBUG_MODE {
            eprintln!("Window error: {}", e);
        }
    });
}
