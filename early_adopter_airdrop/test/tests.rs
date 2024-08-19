use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::solana_program::sysvar::rent;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use solana_program_test::*;
use solana_sdk::{account::Account, signature::Keypair, transaction::Transaction};
use early_adopter_airdrop::program::early_adopter_airdrop;
use early_adopter_airdrop::{self, *};

async fn setup_test() -> (
    ProgramTestContext,
    Keypair,
    Pubkey,
    Keypair,
    Keypair,
    Keypair,
    Pubkey,
) {
    let program_id = Pubkey::new_unique();
    let mint_keypair = Keypair::new();
    let user_keypair = Keypair::new();
    let user_account = Keypair::new();
    let referrer_keypair = Keypair::new();

    let mut test = ProgramTest::new(
        "early_adopter_airdrop",
        program_id,
        processor!(early_adopter_airdrop::entry),
    );

    let context = test.start_with_context().await;

    (context, mint_keypair, program_id, user_keypair, user_account, referrer_keypair, program_id)
}

#[tokio::test]
async fn test_initialize_mint() {
    let (mut context, mint_keypair, program_id, _, _, _, _) = setup_test().await;
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Mint::LEN);

    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &mint_keypair.pubkey(),
                rent_lamports,
                Mint::LEN as u64,
                &token::id(),
            ),
            token::instruction::initialize_mint(
                &token::id(),
                &mint_keypair.pubkey(),
                &context.payer.pubkey(),
                None,
                9,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_keypair],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    let mint_account = context
        .banks_client
        .get_account(mint_keypair.pubkey())
        .await
        .expect("account not found")
        .expect("account empty");

    let mint_info = Mint::unpack(&mint_account.data).unwrap();
    assert_eq!(mint_info.decimals, 9);
    assert_eq!(mint_info.mint_authority, COption::Some(context.payer.pubkey()));
}

#[tokio::test]
async fn test_initialize_user() {
    let (mut context, _, program_id, user_keypair, user_account, _, _) = setup_test().await;

    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    let user_account_data = context
        .banks_client
        .get_account(user_account.pubkey())
        .await
        .expect("account not found")
        .expect("account empty");

    let user_info: UserAccount = UserAccount::try_from_slice(&user_account_data.data).unwrap();
    assert_eq!(user_info.user, user_keypair.pubkey());
    assert_eq!(user_info.loyalty_points, 0);
}

#[tokio::test]
async fn test_reward_early_adopter() {
    let (mut context, mint_keypair, program_id, user_keypair, user_account, _, _) = setup_test().await;

    // Initialize mint
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Mint::LEN);

    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &mint_keypair.pubkey(),
                rent_lamports,
                Mint::LEN as u64,
                &token::id(),
            ),
            token::instruction::initialize_mint(
                &token::id(),
                &mint_keypair.pubkey(),
                &context.payer.pubkey(),
                None,
                9,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_keypair],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Initialize user account
    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Create associated token account for the user
    let user_token_account = token::create_associated_token_account(
        &context.payer.pubkey(),
        &user_keypair.pubkey(),
        &mint_keypair.pubkey(),
    );

    let amount = 1000;
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::reward_early_adopter(
                &program_id,
                &mint_keypair.pubkey(),
                &user_token_account,
                &context.payer.pubkey(),
                amount,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch the user's token account
    let user_token_account_info = context
        .banks_client
        .get_account(user_token_account)
        .await
        .expect("account not found")
        .expect("account empty");

    let user_token_info = TokenAccount::unpack(&user_token_account_info.data).unwrap();
    assert_eq!(user_token_info.amount, amount);
}

#[tokio::test]
async fn test_track_loyalty() {
    let (mut context, _, program_id, user_keypair, user_account, _, _) = setup_test().await;

    // Initialize user account
    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    let points = 50;
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::track_loyalty(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                points,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch the user account to verify loyalty points
    let user_account_data = context
        .banks_client
        .get_account(user_account.pubkey())
        .await
        .expect("account not found")
        .expect("account empty");

    let user_info: UserAccount = UserAccount::try_from_slice(&user_account_data.data).unwrap();
    assert_eq!(user_info.loyalty_points, points);
}

#[tokio::test]
async fn test_burn_tokens() {
    let (mut context, mint_keypair, program_id, user_keypair, user_account, _, _) = setup_test().await;

    // Initialize mint
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_lamports = rent.minimum_balance(Mint::LEN);

    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &mint_keypair.pubkey(),
                rent_lamports,
                Mint::LEN as u64,
                &token::id(),
            ),
            token::instruction::initialize_mint(
                &token::id(),
                &mint_keypair.pubkey(),
                &context.payer.pubkey(),
                None,
                9,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_keypair],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Initialize user account
    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Create associated token account for the user
    let user_token_account = token::create_associated_token_account(
        &context.payer.pubkey(),
        &user_keypair.pubkey(),
        &mint_keypair.pubkey(),
    );

    // Mint tokens to user
    let mint_amount = 1000;
    let tx = Transaction::new_signed_with_payer(
        &[
            token::instruction::mint_to(
                &token::id(),
                &mint_keypair.pubkey(),
                &user_token_account,
                &context.payer.pubkey(),
                &[],
                mint_amount,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Burn tokens from user
    let burn_amount = 500;
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::burn_tokens(
                &program_id,
                &mint_keypair.pubkey(),
                &user_token_account,
                &context.payer.pubkey(),
                burn_amount,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch the user's token account
    let user_token_account_info = context
        .banks_client
        .get_account(user_token_account)
        .await
        .expect("account not found")
        .expect("account empty");

    let user_token_info = TokenAccount::unpack(&user_token_account_info.data).unwrap();
    assert_eq!(user_token_info.amount, mint_amount - burn_amount);
}

#[tokio::test]
async fn test_apply_inactivity_penalty() {
    let (mut context, _, program_id, user_keypair, user_account, _, _) = setup_test().await;

    // Initialize user account
    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Track loyalty points
    let points = 50;
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::track_loyalty(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                points,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Apply inactivity penalty
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::apply_inactivity_penalty(
                &program_id,
                &user_account.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch the user account to verify loyalty points after penalty
    let user_account_data = context
        .banks_client
        .get_account(user_account.pubkey())
        .await
        .expect("account not found")
        .expect("account empty");

    let user_info: UserAccount = UserAccount::try_from_slice(&user_account_data.data).unwrap();
    // Assuming the penalty reduced points by 10
    assert_eq!(user_info.loyalty_points, points - 10);
}

#[tokio::test]
async fn test_update_profile() {
    let (mut context, _, program_id, user_keypair, user_account, _, _) = setup_test().await;

    // Initialize user account
    let tx = Transaction::new_signed_with_payer(
        &[
            system_program::create_account(
                &context.payer.pubkey(),
                &user_account.pubkey(),
                Rent::default().minimum_balance(8 + 32 + 8 + 8 + 4 + 100 + 200),
                8 + 32 + 8 + 8 + 4 + 100 + 200,
                &program_id,
            ),
            early_adopter_airdrop::instruction::initialize_user(
                &program_id,
                &user_account.pubkey(),
                &user_keypair.pubkey(),
                &context.payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &user_account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Update user profile
    let name = "Users Name".to_string();
    let bio = "Users Bio.".to_string();
    let tx = Transaction::new_signed_with_payer(
        &[
            early_adopter_airdrop::instruction::update_profile(
                &program_id,
                &user_account.pubkey(),
                name.clone(),
                bio.clone(),
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Fetch the user account to verify profile update
    let user_account_data = context
        .banks_client
        .get_account(user_account.pubkey())
        .await
        .expect("account not found")
        .expect("account empty");

    let user_info: UserAccount = UserAccount::try_from_slice(&user_account_data.data).unwrap();
    assert_eq!(user_info.name, name);
    assert_eq!(user_info.bio, bio);
}
