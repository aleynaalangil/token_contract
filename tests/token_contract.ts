import * as anchor from '@coral-xyz/anchor';
import type { Program } from '@coral-xyz/anchor';
import { sendAndConfirmTransaction } from '@solana/web3.js';
import type { TokenContract } from '../target/types/token_contract';

describe('anchor', () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenContract as Program<TokenContract>;
  const connection = program.provider.connection;
  const TOKEN_2022_PROGRAM_ID = new anchor.web3.PublicKey('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');
  const wallet = provider.wallet as anchor.Wallet;
  const ATA_PROGRAM_ID = new anchor.web3.PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');

  const tokenName = 'TestToken';
  const [mint] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from('token-2022-token'), wallet.publicKey.toBytes(), Buffer.from(tokenName)],
    program.programId,
  );
  const [payerATA] = anchor.web3.PublicKey.findProgramAddressSync(
    [wallet.publicKey.toBytes(), TOKEN_2022_PROGRAM_ID.toBytes(), mint.toBytes()],
    ATA_PROGRAM_ID,
  );

  const receiver = anchor.web3.Keypair.generate();

  const [receiverATA] = anchor.web3.PublicKey.findProgramAddressSync(
    [receiver.publicKey.toBytes(), TOKEN_2022_PROGRAM_ID.toBytes(), mint.toBytes()],
    ATA_PROGRAM_ID,
  );
  
  it('Add Shareholder by Company', async () => {
    const newShareholder = anchor.web3.Keypair.generate();
    const votingPower = new anchor.BN(1000); // Example voting power
    const [companyAccount] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('company'), wallet.publicKey.toBytes()],
      program.programId
    );
  
    const tx = new anchor.web3.Transaction();
    const ix = await program.methods
      .addShareholderByCompany(newShareholder.publicKey, votingPower)
      .accounts({
        company: companyAccount, // This needs to be defined or fetched
        shareholder: newShareholder.publicKey,
        payer: wallet.publicKey,
      })
      .signers([newShareholder])
      .instruction();
  
    tx.add(ix);
    const sig = await sendAndConfirmTransaction(program.provider.connection, tx, [wallet.payer, newShareholder]);
    console.log('Transaction Signature:', sig);
  });
});