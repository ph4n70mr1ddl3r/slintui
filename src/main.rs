use rand::distributions::uniform::SampleRange;
use rand::{seq::SliceRandom, thread_rng, Rng, RngCore};
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

#[derive(Clone, PartialEq)]
struct Card {
    rank: String,
    suit: String,
    value: i32,
    is_face_up: bool,
}

impl Card {
    fn new(rank: &str, suit: &str, value: i32) -> Self {
        Self {
            rank: rank.to_string(),
            suit: suit.to_string(),
            value,
            is_face_up: false,
        }
    }

    fn to_string(&self) -> String {
        format!("{} of {}", self.rank, self.suit)
    }
}

#[derive(Clone)]
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
            chips: 1000,
            bet: 0,
            cards: Vec::new(),
            is_user,
            last_action: String::new(),
        }
    }
}

#[derive(Clone, PartialEq)]
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
    animation_active: bool,
}

impl PokerGame {
    fn new() -> Self {
        let mut deck = Vec::new();
        let ranks = [
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K", "A",
        ];
        let suits = ["♠", "♥", "♦", "♣"];
        let mut value = 2;
        for rank in &ranks {
            for suit in &suits {
                deck.push(Card::new(rank, suit, value));
            }
            value += 1;
        }

        let mut players = Vec::new();
        players.push(Player::new("You", true));
        players.push(Player::new("Bot", false));

        Self {
            deck,
            community_cards: Vec::new(),
            players,
            current_player: 0,
            phase: GamePhase::PreFlop,
            pot: 0,
            current_bet: 0,
            dealer_position: 0,
            small_blind: 10,
            big_blind: 20,
            animation_active: false,
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
        self.shuffle_deck();
        self.community_cards.clear();
        self.pot = 0;
        self.current_bet = 0;

        for player in &mut self.players {
            player.bet = 0;
            player.cards.clear();
            player.last_action = String::new();
        }

        let mut new_cards: Vec<Vec<Card>> = vec![Vec::new(), Vec::new()];
        for _ in 0..2 {
            for i in 0..self.players.len() {
                if let Some(card) = self.deal_card() {
                    new_cards[i].push(card);
                }
            }
        }

        for (i, player) in self.players.iter_mut().enumerate() {
            player.cards = new_cards[i].clone();
        }

        self.post_blinds();
        self.phase = GamePhase::PreFlop;
        self.current_player = (self.dealer_position + 3) % self.players.len();
    }

    fn post_blinds(&mut self) {
        let sb_player = (self.dealer_position + 1) % self.players.len();
        let bb_player = (self.dealer_position + 2) % self.players.len();

        self.players[sb_player].bet = self.small_blind;
        self.players[sb_player].chips -= self.small_blind;
        self.players[sb_player].last_action = format!("Small Blind: {}", self.small_blind);

        self.players[bb_player].bet = self.big_blind;
        self.players[bb_player].chips -= self.big_blind;
        self.players[bb_player].last_action = format!("Big Blind: {}", self.big_blind);

        self.current_bet = self.big_blind;
        self.pot += self.small_blind + self.big_blind;
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
                self.phase = GamePhase::Flop;
                self.deal_community_cards(3);
            }
            GamePhase::Flop => {
                self.phase = GamePhase::Turn;
                self.deal_community_cards(1);
            }
            GamePhase::Turn => {
                self.phase = GamePhase::River;
                self.deal_community_cards(1);
            }
            GamePhase::River => {
                self.phase = GamePhase::Showdown;
            }
            GamePhase::Showdown => {
                self.start_hand();
            }
        }
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

    fn all_players_acted(&self) -> bool {
        self.players
            .iter()
            .all(|p| p.bet == self.current_bet || p.cards.is_empty())
    }

    fn move_to_next_player(&mut self) {
        self.current_player = self.get_next_player();
        if self.all_players_acted() {
            self.next_phase();
        }
    }

    fn player_action(&mut self, action: &str, amount: Option<i32>) -> bool {
        let player = &mut self.players[self.current_player];
        let bet_amount = amount.unwrap_or(0);

        match action {
            "fold" => {
                player.cards.clear();
                player.last_action = "Folded".to_string();
                self.move_to_next_player();
                return true;
            }
            "check" => {
                if player.bet >= self.current_bet {
                    player.last_action = "Checked".to_string();
                    self.move_to_next_player();
                    return true;
                }
            }
            "bet" | "raise" => {
                let to_bet = bet_amount.max(self.current_bet + 20);
                if player.chips >= to_bet {
                    player.chips -= to_bet - player.bet;
                    player.bet = to_bet;
                    player.last_action =
                        format!("{action}: {}", to_bet - player.bet + self.current_bet);
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
                    player.last_action = format!("Called: {}", call_amount);
                    self.pot += call_amount;
                    self.move_to_next_player();
                    return true;
                }
            }
            "all-in" => {
                let all_in = player.chips;
                player.chips = 0;
                player.bet += all_in;
                player.last_action = format!("All-In: {}", all_in);
                self.pot += all_in;
                if player.bet > self.current_bet {
                    self.current_bet = player.bet;
                }
                self.move_to_next_player();
                return true;
            }
            _ => {}
        }
        false
    }

    fn simulate_user_action(&mut self) -> bool {
        if !self.players[self.current_player].is_user {
            return false;
        }

        let player_chips = self.players[self.current_player].chips;
        let call_amount = self.current_bet - self.players[self.current_player].bet;
        let to_call = call_amount.max(0);

        let actions = match self.phase {
            GamePhase::PreFlop if to_call == 0 => vec!["check", "bet", "raise", "fold"],
            GamePhase::PreFlop => vec!["call", "raise", "fold", "check"],
            GamePhase::Flop if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::Flop => vec!["call", "raise", "fold"],
            GamePhase::Turn if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::Turn => vec!["call", "raise", "fold"],
            GamePhase::River if to_call == 0 => vec!["check", "bet", "fold"],
            GamePhase::River => vec!["call", "raise", "fold"],
            GamePhase::Showdown => vec![],
        };

        if actions.is_empty() {
            return false;
        }

        let mut rng = thread_rng();
        let action = actions.choose(&mut rng).unwrap();
        let bet_amount = rng.gen_range(50..=player_chips.min(200));

        self.player_action(action, Some(bet_amount))
    }

    fn simulate_bot_action(&mut self) -> bool {
        if self.players[self.current_player].is_user {
            return false;
        }

        let player_chips = self.players[self.current_player].chips;
        let call_amount = self.current_bet - self.players[self.current_player].bet;
        let to_call = call_amount.max(0);

        let mut rng = thread_rng();
        let bot_aggressive = rng.gen::<f64>() < 0.4;
        let bot_bluff = rng.gen::<f64>() < 0.15;

        if bot_bluff && to_call > 0 && player_chips >= to_call {
            return self.player_action("raise", Some(rng.gen_range(50..=player_chips.min(150))));
        }

        if to_call == 0 {
            if bot_aggressive {
                let bet = rng.gen_range(20..=player_chips.min(100));
                return self.player_action("bet", Some(bet));
            } else {
                return self.player_action("check", None);
            }
        }

        if player_chips <= to_call {
            return self.player_action("call", None);
        }

        if bot_aggressive && player_chips > to_call + 50 {
            return self.player_action("raise", Some(rng.gen_range(30..=player_chips.min(120))));
        }

        if to_call == 0 {
            if bot_aggressive {
                let bet = rng.gen_range(20i32..=player_chips.min(100));
                return self.player_action("bet", Some(bet));
            } else {
                return self.player_action("check", None);
            }
        }

        if player_chips <= to_call {
            return self.player_action("call", None);
        }

        if bot_aggressive && player_chips > to_call + 50 {
            return self.player_action("raise", Some(rng.gen_range(30i32..=player_chips.min(120))));
        }

        if to_call > player_chips / 3 {
            return self.player_action("fold", None);
        }

        self.player_action("call", None)
    }

    fn any_player_active(&self) -> bool {
        self.players.iter().any(|p| !p.cards.is_empty())
    }

    fn get_winner(&self) -> usize {
        let mut best_score = 0;
        let mut winner = 0;

        for (i, player) in self.players.iter().enumerate() {
            if player.cards.is_empty() {
                continue;
            }
            let score = self.calculate_hand_score(player);
            if score > best_score {
                best_score = score;
                winner = i;
            }
        }

        winner
    }

    fn calculate_hand_score(&self, player: &Player) -> i32 {
        let mut all_cards = player.cards.clone();
        all_cards.extend(self.community_cards.clone());

        let mut total = 0;
        for card in &all_cards {
            total += card.value;
        }
        total
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
        visible: card.is_face_up,
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
        state.game.borrow_mut().start_hand();
        state
    }

    fn update_ui(&self) {
        let game = self.game.borrow();
        let window = self.main_window.upgrade().unwrap();

        window.set_pot(game.pot);
        window.set_current_bet(game.current_bet);
        window.set_phase_name(game.get_phase_name().into());
        window.set_current_player_name(game.players[game.current_player].name.clone().into());

        let player_cards: Vec<CardUI> = game.players[0]
            .cards
            .iter()
            .map(create_card_ui_data)
            .collect();
        window.set_player_cards(Rc::new(VecModel::from(player_cards)).into());

        let bot_cards: Vec<CardUI> = game.players[1]
            .cards
            .iter()
            .map(create_card_ui_data)
            .collect();
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

        let is_user_turn =
            game.players[game.current_player].is_user && game.phase != GamePhase::Showdown;
        let call_amount = game.current_bet - game.players[0].bet;
        let can_check = call_amount <= 0;
        let can_call = game.players[0].chips >= call_amount.max(0);
        let min_raise = game.current_bet + 20;

        window.set_show_actions(is_user_turn);
        window.set_can_check(can_check);
        window.set_can_call(can_call);
        window.set_can_fold(true);
        window.set_can_raise(game.players[0].chips >= min_raise);
        window.set_min_raise_amount(min_raise);

        if game.phase == GamePhase::Showdown {
            let winner = game.get_winner();
            let winner_name = game.players[winner].name.clone();
            window.set_show_winner(true);
            window.set_winner_name(winner_name.into());
        } else {
            window.set_show_winner(false);
        }
    }

    fn handle_action(&self, action: &str) {
        let mut game = self.game.borrow_mut();
        if !game.players[game.current_player].is_user {
            return;
        }

        let amount = match action {
            "raise" | "bet" => Some(50),
            "all-in" => Some(game.players[0].chips),
            _ => None,
        };

        if game.player_action(action, amount) {
            drop(game);
            self.update_ui();
            self.run_simulation();
        }
    }

    fn run_simulation(&self) {
        let state = Rc::new(self.clone());
        let window = self.main_window.upgrade().unwrap();
        window.set_animation_active(true);

        let state_clone = state.clone();
        let window_weak = self.main_window.clone();
        let timer = Rc::new(RefCell::new(slint::Timer::default()));

        let mut step = 0;

        let timer_clone = timer.clone();
        timer.borrow_mut().start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(800),
            move || {
                let window = match window_weak.upgrade() {
                    Some(w) => w,
                    None => {
                        timer_clone.borrow_mut().stop();
                        return;
                    }
                };

                let mut game = state_clone.game.borrow_mut();

                if step >= 10 {
                    drop(game);
                    window.set_animation_active(false);
                    timer_clone.borrow_mut().stop();
                    return;
                }

                if game.phase == GamePhase::Showdown {
                    drop(game);
                    state_clone.update_ui();
                    window.set_animation_active(false);
                    step = 0;
                    timer_clone.borrow_mut().stop();
                    return;
                }

                if game.any_player_active() {
                    if game.players[game.current_player].is_user {
                        game.simulate_user_action();
                    } else {
                        game.simulate_bot_action();
                    }

                    drop(game);
                    state_clone.update_ui();
                } else {
                    if game.phase == GamePhase::PreFlop {
                        drop(game);
                        state_clone.update_ui();
                        window.set_animation_active(false);
                        step = 0;
                        timer_clone.borrow_mut().stop();
                        return;
                    }
                }

                step += 1;
            },
        );
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
    let main_window = MainWindow::new().unwrap();

    let weak_window = main_window.as_weak();
    let state = Rc::new(AppState::new(weak_window.clone()));
    state.update_ui();

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_check(move || {
        state_clone.handle_action("check");
    });

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_call(move || {
        state_clone.handle_action("call");
    });

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_fold(move || {
        state_clone.handle_action("fold");
    });

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_raise(move || {
        state_clone.handle_action("raise");
    });

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_all_in(move || {
        state_clone.handle_action("all-in");
    });

    let state_clone = state.clone();
    let window = weak_window.upgrade().unwrap();

    window.on_new_hand(move || {
        let mut game = state_clone.game.borrow_mut();
        game.start_hand();
        drop(game);
        state_clone.update_ui();
        state_clone.run_simulation();
    });

    state.run_simulation();

    main_window.run().unwrap();
}
