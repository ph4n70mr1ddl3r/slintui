use rand::{seq::SliceRandom, thread_rng, Rng};
use slint::{ComponentHandle, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

slint::include_modules!();

#[derive(Clone)]
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

#[derive(Clone, PartialEq, Debug)]
enum GamePhase {
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    HandComplete,
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
    cards_dealt_this_round: usize,
    hand_complete: bool,
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
            cards_dealt_this_round: 0,
            hand_complete: false,
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

    fn deal_hole_cards(&mut self) {
        println!("  Dealing hole cards...");
        for _ in 0..2 {
            for i in 0..self.players.len() {
                if let Some(card) = self.deal_card() {
                    println!(
                        "    {} receives {} of {}",
                        self.players[i].name, card.rank, card.suit
                    );
                    self.players[i].cards.push(card);
                }
            }
        }
    }

    fn deal_community_card(&mut self) -> bool {
        if let Some(card) = self.deal_card() {
            println!(
                "  Dealing community card: {} of {} ({} community cards total)",
                card.rank,
                card.suit,
                self.community_cards.len() + 1
            );
            self.community_cards.push(card);
            true
        } else {
            false
        }
    }

    fn start_hand(&mut self) {
        println!("\n========================================");
        println!("STARTING NEW HAND");
        println!("========================================");

        self.create_deck();
        self.shuffle_deck();
        self.community_cards.clear();
        self.pot = 0;
        self.current_bet = 0;
        self.phase = GamePhase::PreFlop;
        self.cards_dealt_this_round = 0;
        self.hand_complete = false;

        for player in &mut self.players {
            player.bet = 0;
            player.cards.clear();
            player.last_action = String::new();
        }

        println!("Dealer position: {}", self.dealer_position);
        println!(
            "{} is SB ({}), {} is BB ({})",
            self.players[(self.dealer_position + 1) % 2].name,
            self.small_blind,
            self.players[(self.dealer_position + 2) % 2].name,
            self.big_blind
        );

        self.post_blinds();
        println!("Pot after blinds: ${}", self.pot);

        self.deal_hole_cards();

        self.current_player = (self.dealer_position + 3) % self.players.len();
        println!(
            "Current player: {} ({}'s turn)",
            self.current_player, self.players[self.current_player].name
        );
    }

    fn post_blinds(&mut self) {
        let sb_player = (self.dealer_position + 1) % self.players.len();
        let bb_player = (self.dealer_position + 2) % self.players.len();

        self.players[sb_player].bet = self.small_blind;
        self.players[sb_player].chips -= self.small_blind;
        self.players[sb_player].last_action = format!("SB: {}", self.small_blind);
        println!(
            "{} posts SB: ${}",
            self.players[sb_player].name, self.small_blind
        );

        self.players[bb_player].bet = self.big_blind;
        self.players[bb_player].chips -= self.big_blind;
        self.players[bb_player].last_action = format!("BB: {}", self.big_blind);
        println!(
            "{} posts BB: ${}",
            self.players[bb_player].name, self.big_blind
        );

        self.current_bet = self.big_blind;
        self.pot += self.small_blind + self.big_blind;
    }

    fn next_phase(&mut self) {
        match self.phase {
            GamePhase::PreFlop => {
                println!("\n--- PHASE: FLOP ---");
                println!("Dealing 3 community cards...");
                for _ in 0..3 {
                    self.deal_community_card();
                }
                self.phase = GamePhase::Flop;
                self.cards_dealt_this_round = 0;
            }
            GamePhase::Flop => {
                println!("\n--- PHASE: TURN ---");
                println!("Dealing 1 community card...");
                self.deal_community_card();
                self.phase = GamePhase::Turn;
            }
            GamePhase::Turn => {
                println!("\n--- PHASE: RIVER ---");
                println!("Dealing 1 community card...");
                self.deal_community_card();
                self.phase = GamePhase::River;
            }
            GamePhase::River => {
                println!("\n--- PHASE: SHOWDOWN ---");
                self.phase = GamePhase::Showdown;
            }
            GamePhase::Showdown => {
                self.hand_complete = true;
            }
            GamePhase::HandComplete => {}
        }
        println!("Pot: ${}", self.pot);
    }

    fn get_phase_name(&self) -> String {
        match self.phase {
            GamePhase::PreFlop => "Pre-Flop".to_string(),
            GamePhase::Flop => "Flop".to_string(),
            GamePhase::Turn => "Turn".to_string(),
            GamePhase::River => "River".to_string(),
            GamePhase::Showdown => "Showdown!".to_string(),
            GamePhase::HandComplete => "Hand Complete".to_string(),
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
                println!("{} FOLDS!", player.name);
                player.cards.clear();
                player.last_action = "Folded".to_string();
                self.move_to_next_player();
                return true;
            }
            "check" => {
                if player.bet >= self.current_bet {
                    println!("{} CHECKS", player.name);
                    player.last_action = "Check".to_string();
                    self.move_to_next_player();
                    return true;
                }
            }
            "bet" | "raise" => {
                let to_bet = bet_amount.max(self.current_bet + 20);
                if player.chips >= to_bet {
                    player.chips -= to_bet - player.bet;
                    player.bet = to_bet;
                    let action_type = if action == "bet" { "BETS" } else { "RAISES" };
                    println!(
                        "{} {} ${} (total bet: ${})",
                        player.name,
                        action_type,
                        to_bet - player.bet + if player.bet > 0 { player.bet } else { 0 },
                        to_bet
                    );
                    player.last_action = format!("Bet: {}", to_bet);
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
                    println!("{} CALLS ${}", player.name, call_amount);
                    player.last_action = format!("Call: {}", call_amount);
                    self.pot += call_amount;
                    self.move_to_next_player();
                    return true;
                }
            }
            "all-in" => {
                let all_in = player.chips;
                player.chips = 0;
                player.bet += all_in;
                println!("{} GOES ALL-IN FOR ${}!", player.name, all_in);
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

    fn simulate_bot_action(&mut self) {
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
            GamePhase::HandComplete => vec![],
        };

        if actions.is_empty() {
            return;
        }

        let action = actions.choose(&mut rng).unwrap();
        let bet_amount = rng.gen_range(50..=player_chips.min(200));

        println!("[BOT THINKING...]");
        self.player_action(action, Some(bet_amount));
    }

    fn get_winner(&self) -> usize {
        let active_players: Vec<(usize, &Player)> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.cards.is_empty())
            .collect();

        if active_players.len() == 1 {
            println!(
                "Only 1 player remaining - {} wins by default!",
                active_players[0].1.name
            );
            return active_players[0].0;
        }

        println!("\n--- SHOWDOWN ---");
        println!(
            "Your hand: {} {} = {} pts",
            self.players[0].cards[0].rank,
            self.players[0].cards[0].suit,
            self.players[0].cards[0].value + self.players[0].cards[1].value
        );
        println!(
            "Bot hand: {} {} {} {} = {} pts",
            self.players[1].cards[0].rank,
            self.players[1].cards[0].suit,
            self.players[1].cards[1].rank,
            self.players[1].cards[1].suit,
            self.players[1].cards[0].value + self.players[1].cards[1].value
        );

        for card in &self.community_cards {
            println!(
                "  Community: {} {} ({} pts)",
                card.rank, card.suit, card.value
            );
        }

        let mut best_score = -1;
        let mut winner = 0;

        for (i, player) in &active_players {
            let score = self.calculate_hand_score(player);
            println!("{} total hand score: {}", player.name, score);
            if score > best_score {
                best_score = score;
                winner = *i;
            }
        }

        winner
    }

    fn award_pot(&mut self) {
        let winner = self.get_winner();
        self.players[winner].chips += self.pot;
        println!(
            "\n{} WINS THE POT OF ${}!",
            self.players[winner].name, self.pot
        );
        println!(
            "{} now has ${}",
            self.players[winner].name, self.players[winner].chips
        );
        println!(
            "{} has ${}",
            if winner == 0 {
                &self.players[1].name
            } else {
                &self.players[0].name
            },
            if winner == 0 {
                self.players[1].chips
            } else {
                self.players[0].chips
            }
        );
        println!("========================================\n");
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

    fn active_players(&self) -> usize {
        self.players.iter().filter(|p| !p.cards.is_empty()).count()
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
        state
    }

    fn update_ui(&self) {
        let game = self.game.borrow();
        let window = self.main_window.upgrade().unwrap();

        window.set_pot(game.pot);
        window.set_current_bet(game.current_bet);
        window.set_phase_name(game.get_phase_name().into());
        window.set_current_player_name(game.players[game.current_player].name.clone().into());
        window.set_game_over(game.hand_complete);

        let player_cards: Vec<CardUI> = game.players[0]
            .cards
            .iter()
            .map(create_card_ui_data)
            .collect();
        window.set_player_cards(Rc::new(VecModel::from(player_cards)).into());

        let bot_cards: Vec<CardUI> = game.players[1]
            .cards
            .iter()
            .map(|c| CardUI {
                rank: c.rank.clone().into(),
                suit: c.suit.clone().into(),
                card_color: if c.suit == "♥" || c.suit == "♦" {
                    "red".into()
                } else {
                    "black".into()
                },
                visible: game.hand_complete || game.phase == GamePhase::Showdown,
            })
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

        let is_user_turn = game.is_user_turn();
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

        if game.phase == GamePhase::Showdown && !game.hand_complete {
            let winner = game.get_winner();
            let winner_name = game.players[winner].name.clone();
            window.set_show_winner(true);
            window.set_winner_name(format!("{} wins {}!", winner_name, game.pot).into());
        } else {
            window.set_show_winner(false);
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
    println!("Starting poker app...");
    let main_window = MainWindow::new().unwrap();
    println!("MainWindow created");
    let weak_window = main_window.as_weak();

    let state = Rc::new(AppState::new(weak_window.clone()));

    {
        let mut game = state.game.borrow_mut();
        game.start_hand();
    }
    println!("Initial hand dealt");
    state.update_ui();

    let process_bot_turn = |state: &AppState| {
        let mut game = state.game.borrow_mut();
        while game.is_bot_turn()
            && game.active_players() > 1
            && !game.hand_complete
            && game.phase != GamePhase::Showdown
        {
            println!("[AUTO] Bot's turn in {}", game.get_phase_name());
            game.simulate_bot_action();
            if game.phase == GamePhase::Showdown && game.community_cards.len() == 5 {
                game.hand_complete = true;
                game.award_pot();
            }
            drop(game);
            state.update_ui();
            thread::sleep(Duration::from_millis(800));
            game = state.game.borrow_mut();
        }
    };

    let state_check = state.clone();
    main_window.on_check(move || {
        println!("CHECK CLICKED!");
        let mut game = state_check.game.borrow_mut();
        if game.player_action("check", None) {
            drop(game);
            state_check.update_ui();
            process_bot_turn(&state_check);
        }
    });

    let state_call = state.clone();
    main_window.on_call(move || {
        println!("CALL CLICKED!");
        let mut game = state_call.game.borrow_mut();
        if game.player_action("call", None) {
            drop(game);
            state_call.update_ui();
            process_bot_turn(&state_call);
        }
    });

    let state_fold = state.clone();
    main_window.on_fold(move || {
        println!("FOLD CLICKED!");
        let mut game = state_fold.game.borrow_mut();
        if game.player_action("fold", None) {
            drop(game);
            state_fold.update_ui();
            process_bot_turn(&state_fold);
        }
    });

    let state_raise = state.clone();
    main_window.on_raise(move || {
        println!("RAISE CLICKED!");
        let mut game = state_raise.game.borrow_mut();
        let min_raise = game.current_bet + 20;
        if game.player_action("raise", Some(min_raise)) {
            drop(game);
            state_raise.update_ui();
            process_bot_turn(&state_raise);
        }
    });

    let state_all_in = state.clone();
    main_window.on_all_in(move || {
        println!("ALL-IN CLICKED!");
        let mut game = state_all_in.game.borrow_mut();
        if game.player_action("all-in", None) {
            drop(game);
            state_all_in.update_ui();
            process_bot_turn(&state_all_in);
        }
    });

    let state_new = state.clone();
    main_window.on_new_hand(move || {
        println!("\n\n>>> NEW HAND CLICKED <<<\n");
        {
            let mut game = state_new.game.borrow_mut();
            game.start_hand();
        }
        state_new.update_ui();

        thread::sleep(Duration::from_millis(500));

        let mut round = 0;
        loop {
            round += 1;

            let (phase_name, current_player_name, current_player_idx);
            {
                let game = state_new.game.borrow();
                phase_name = game.get_phase_name();
                current_player_idx = game.current_player;
                current_player_name = game.players[game.current_player].name.clone();
            }

            println!(
                "\n[Round {}] Phase: {}, Current player: {}",
                round, phase_name, current_player_name
            );

            let (hand_complete, phase, community_cards_len, active_players_count, is_user);
            {
                let game = state_new.game.borrow();
                hand_complete = game.hand_complete;
                phase = game.phase.clone();
                community_cards_len = game.community_cards.len();
                active_players_count = game.active_players();
                is_user = game.is_user_turn();
            }

            if hand_complete || phase == GamePhase::Showdown {
                println!("Hand complete or at showdown, breaking...");
                if phase == GamePhase::Showdown && community_cards_len == 5 && !hand_complete {
                    println!("Awarding pot at showdown...");
                    let mut game = state_new.game.borrow_mut();
                    game.hand_complete = true;
                    game.award_pot();
                }
                break;
            }

            if active_players_count <= 1 {
                println!(
                    "Only {} player(s) remaining, ending hand",
                    active_players_count
                );
                {
                    let mut game = state_new.game.borrow_mut();
                    game.hand_complete = true;
                    if active_players_count == 1 {
                        game.award_pot();
                    }
                }
                break;
            }

            if is_user {
                println!("It's YOUR turn - waiting for action");
                break;
            }

            println!("Bot is acting...");
            {
                let mut game = state_new.game.borrow_mut();
                game.simulate_bot_action();

                if game.phase == GamePhase::Showdown && game.community_cards.len() == 5 {
                    println!("Reached showdown with all community cards");
                    game.hand_complete = true;
                    game.award_pot();
                }
            }

            state_new.update_ui();
            thread::sleep(Duration::from_millis(800));
        }

        println!("Updating UI one final time...");
        state_new.update_ui();
    });

    println!("Callbacks set up, entering event loop...");
    main_window.run().unwrap();
}
