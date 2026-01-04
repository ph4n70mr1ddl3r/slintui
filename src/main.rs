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
        println!("\n=== STARTING NEW HAND ===");

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

        println!(
            "You: ${}  |  Bot: ${}",
            self.players[0].chips, self.players[1].chips
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

        println!(
            "Dealer: {}  |  SB: {}  |  BB: {}",
            dealer_name, sb_name, bb_name
        );

        self.post_blinds();
        self.deal_hole_cards();

        self.current_player = (self.dealer_position + 3) % self.players.len();
        println!(
            "\n>>> {}'s turn ({})",
            self.players[self.current_player].name,
            self.get_phase_name()
        );
        println!("Pot: ${}  |  Current bet: ${}", self.pot, self.current_bet);
    }

    fn post_blinds(&mut self) {
        let sb_player = (self.dealer_position + 1) % self.players.len();
        let bb_player = (self.dealer_position + 2) % self.players.len();

        self.players[sb_player].bet = self.small_blind;
        self.players[sb_player].chips -= self.small_blind;
        self.players[sb_player].last_action = format!("SB: ${}", self.small_blind);
        println!(
            "  {} posts small blind: ${}",
            self.players[sb_player].name, self.small_blind
        );

        self.players[bb_player].bet = self.big_blind;
        self.players[bb_player].chips -= self.big_blind;
        self.players[bb_player].last_action = format!("BB: ${}", self.big_blind);
        println!(
            "  {} posts big blind: ${}",
            self.players[bb_player].name, self.big_blind
        );

        self.current_bet = self.big_blind;
        self.pot += self.small_blind + self.big_blind;
    }

    fn deal_hole_cards(&mut self) {
        println!("\n Dealing hole cards...");
        for i in 0..self.players.len() {
            if let Some(card) = self.deal_card() {
                self.players[i].cards.push(card.clone());
            }
            if let Some(card) = self.deal_card() {
                self.players[i].cards.push(card.clone());
            }
            if self.players[i].is_user {
                println!(
                    "  Your cards: {} {} | {} {}",
                    self.players[i].cards[0].rank,
                    self.players[i].cards[0].suit,
                    self.players[i].cards[1].rank,
                    self.players[i].cards[1].suit
                );
            } else {
                println!("  Bot cards: [hidden] [hidden]");
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
                println!("\n=== THE FLOP ===");
                self.deal_community_cards(3);
                self.phase = GamePhase::Flop;
            }
            GamePhase::Flop => {
                println!("\n=== THE TURN ===");
                self.deal_community_cards(1);
                self.phase = GamePhase::Turn;
            }
            GamePhase::Turn => {
                println!("\n=== THE RIVER ===");
                self.deal_community_cards(1);
                self.phase = GamePhase::River;
            }
            GamePhase::River => {
                println!("\n=== SHOWDOWN ===");
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
        println!("\nCommunity cards: {}", community_str);
        println!("\n>>> {}'s turn", self.players[self.current_player].name);
        println!("Pot: ${}  |  Current bet: $0", self.pot);
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
                println!("  {} FOLDS!", player.name);
                player.cards.clear();
                player.last_action = "Folded".to_string();
                self.move_to_next_player();
                return true;
            }
            "check" => {
                if player.bet >= self.current_bet {
                    println!("  {} CHECKS", player.name);
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
                    println!("  {} {} ${}", player.name, action_type, actual_bet);
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
                    println!("  {} CALLS ${}", player.name, call_amount);
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
                    println!("  {} GOES ALL-IN FOR ${}!", player.name, all_in);
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
        let call_amount = self.current_bet - self.players[self.current_player].bet;
        let to_call = call_amount.max(0);

        let mut rng = thread_rng();

        let actions = match self.phase {
            GamePhase::PreFlop if to_call == 0 => vec!["check", "bet", "raise", "fold"],
            GamePhase::PreFlop => vec!["call", "raise", "fold"],
            GamePhase::Flop if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::Flop => vec!["call", "raise", "fold"],
            GamePhase::Turn if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::Turn => vec!["call", "raise", "fold"],
            GamePhase::River if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::River => vec!["call", "raise", "fold"],
            GamePhase::Showdown => vec![],
        };

        if actions.is_empty() {
            return;
        }

        let action = actions.choose(&mut rng).unwrap();
        let bet_amount = match *action {
            "bet" | "raise" => rng.gen_range(MIN_BET_AMOUNT..=player_chips.min(MAX_BET_AMOUNT)),
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

        println!("\n=== SHOWDOWN RESULTS ===");

        let user = &self.players[0];
        let bot = &self.players[1];

        if user.cards.len() >= 2 {
            println!(
                "\n Your hand: {} {} | {} {} (score: {})",
                user.cards[0].rank,
                user.cards[0].suit,
                user.cards[1].rank,
                user.cards[1].suit,
                user.cards[0].value + user.cards[1].value
            );
        } else {
            println!("\n Your hand: (folded)");
        }

        if !bot.cards.is_empty() && bot.cards.len() >= 2 {
            println!(
                " Bot hand: {} {} | {} {} (score: {})",
                bot.cards[0].rank,
                bot.cards[0].suit,
                bot.cards[1].rank,
                bot.cards[1].suit,
                bot.cards[0].value + bot.cards[1].value
            );
        } else if bot.cards.is_empty() {
            println!(" Bot folded!");
        } else {
            println!(" Bot hand: (incomplete)");
        }

        let active_players: Vec<(usize, &Player)> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.cards.is_empty())
            .collect();

        if active_players.len() == 1 {
            let winner_idx = active_players[0].0;
            println!(
                "\n  {} WINS ${} BY DEFAULT!",
                active_players[0].1.name, self.pot
            );
            self.players[winner_idx].chips += self.pot;
            self.game_over = true;
        } else if active_players.len() == 2 {
            let user_score = if user.cards.len() >= 2 {
                user.cards[0].value + user.cards[1].value
            } else {
                0
            };
            let bot_score = if bot.cards.len() >= 2 {
                bot.cards[0].value + bot.cards[1].value
            } else {
                0
            };

            for card in &self.community_cards {
                println!("   + {} {} ({} pts)", card.rank, card.suit, card.value);
            }

            println!("\n  Your total: {} pts", user_score);
            println!("  Bot total:  {} pts", bot_score);

            if user_score > bot_score {
                println!("\n  YOU WIN ${}!", self.pot);
                self.players[0].chips += self.pot;
            } else if bot_score > user_score {
                println!("\n  BOT WINS ${}!", self.pot);
                self.players[1].chips += self.pot;
            } else {
                println!("\n  SPLIT POT! Each gets ${}", self.pot / 2);
                self.players[0].chips += self.pot / 2;
                self.players[1].chips += self.pot / 2;
            }
        }

        self.hand_complete = true;

        println!(
            "\nYour chips: ${}  |  Bot chips: ${}",
            self.players[0].chips, self.players[1].chips
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
            println!("Pot: ${}", game.pot);
            game.check_phase_complete();
            let needs_bot = game.is_bot_turn();
            drop(game);
            self.update_ui();
            if needs_bot {
                self.process_bot_turn();
            }
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
    println!("TEXAS HOLD'EM POKER vs BOT");

    let main_window = match MainWindow::new() {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Failed to create window: {}", e);
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

    println!("\nClick NEW HAND to start playing!");

    let state_check = state.clone();
    main_window.on_check(move || {
        println!("\n>>> You CHECK");
        state_check.process_action("check", None);
    });

    let state_call = state.clone();
    main_window.on_call(move || {
        println!("\n>>> You CALL");
        state_call.process_action("call", None);
    });

    let state_fold = state.clone();
    main_window.on_fold(move || {
        println!("\n>>> You FOLD");
        state_fold.process_action("fold", None);
    });

    let state_raise = state.clone();
    main_window.on_raise(move || {
        let amount = {
            let game = state_raise.game.borrow();
            game.current_bet + MIN_RAISE
        };
        println!("\n>>> You RAISE to ${}", amount);
        state_raise.process_action("raise", Some(amount));
    });

    let state_all_in = state.clone();
    main_window.on_all_in(move || {
        println!("\n>>> You GO ALL-IN!");
        state_all_in.process_action("all-in", None);
    });

    let state_new = state.clone();
    main_window.on_new_hand(move || {
        println!("\n=== NEW HAND ===");
        let show_winner: Option<(String, bool)> = {
            let mut game = state_new.game.borrow_mut();
            if game.is_game_over() {
                println!("\n=== GAME OVER ===");
                let winner = game.get_winner_name();
                let was_game_over = game.game_over;
                println!("{}", winner);

                if was_game_over {
                    game.players[0].chips = 1000;
                    game.players[1].chips = 1000;
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

    main_window.run().unwrap();
}
