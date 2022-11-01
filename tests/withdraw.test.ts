import * as anchor from "@project-serum/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { Program } from "@project-serum/anchor";
import { TokenVestingProgram } from "../target/types/token_vesting_program";
import moment from "moment";
import * as spl from '@solana/spl-token';
import { expect } from "chai";
import { COMMITMENT, PDAAccounts, makeParams, ONE_DAY_IN_SECONDS, ParsedTokenTransfer, createMint, createTokenAccount, getPDAs } from "./utils";


function delay(ms: number) {
    return new Promise( resolve => setTimeout(resolve, ms) );
  }

describe("Withdraw", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const { connection } = provider;
    const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

    it('correctly initializes a new account and transfers the funds', async () => {
        const employer = provider.wallet.publicKey;
        const employeeKeypair = Keypair.generate();
        const employee = employeeKeypair.publicKey;
        const { grant, escrowAuthority, escrowTokenAccount } = await getPDAs({
          employer,
          employee,
          programId: program.programId,
        });
        const mint = await createMint(provider);
        const employerAccount = await createTokenAccount(provider, provider.wallet.publicKey, mint, 100_000 * LAMPORTS_PER_SOL);
        const employeeAccount = await createTokenAccount(provider, employee, mint);

        const twoYearsAgo = moment().subtract(2, 'years');
        const params = makeParams(twoYearsAgo, 12, 4, 1, '40000');
        console.log(JSON.stringify(params));
        const initializeTransaction = await program.methods
            .initialize(params)
            .accounts({
                employee,
                employer,
                grant,
                escrowAuthority,
                escrowTokenAccount,
                mint,
                employerAccount,
            })
            .rpc(COMMITMENT);
        console.log(`[Initialize] ${initializeTransaction}`);

        // Start the withdraw process
        const withdrawTransaction = await program.methods
            .withdraw()
            .accounts({
                employee,
                employer,
                grant,
                escrowAuthority,
                escrowTokenAccount,
                employeeAccount,
            })
            .signers([employeeKeypair])
            .rpc(COMMITMENT);
        const tx = await connection.getParsedTransaction(
          withdrawTransaction,
          COMMITMENT
        );
        console.log(tx.meta.logMessages)
        expect(tx.meta.innerInstructions[0].instructions[0].programId.toBase58()).eql(spl.TOKEN_PROGRAM_ID.toBase58())
        expect((tx.meta.innerInstructions[0].instructions[0] as any).parsed.type).eql("transfer");
        const result: ParsedTokenTransfer = (tx.meta.innerInstructions[0].instructions[0] as any).parsed.info
        expect(result.source).to.eq(escrowTokenAccount.toBase58())
        expect(result.destination).to.eq(employeeAccount.toBase58())
        expect(result.authority).to.eq(escrowAuthority.toBase58())
        
        // Due to local validator delay + delay in processing time from this unit test, make sure grant amout is within ballpark what
        // we expect to get issued that day.
        const expectedAmount = 20_000;
        expect(parseInt(result.amount) / LAMPORTS_PER_SOL).gte(expectedAmount - 30);
        expect(parseInt(result.amount) / LAMPORTS_PER_SOL).lte(expectedAmount + 30);
        console.log(JSON.stringify(result));

        // Check data
        const grantData = await program.account.grant.fetch(grant, 'confirmed');
        expect(grantData.alreadyIssuedTokenAmount.toString()).to.eq(result.amount);
    });
});