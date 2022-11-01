import * as anchor from "@project-serum/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { Program } from "@project-serum/anchor";
import { TokenVestingProgram } from "../target/types/token_vesting_program";
import * as spl from '@solana/spl-token';
import { expect } from "chai";
import { COMMITMENT, PDAAccounts, makeParams, ONE_DAY_IN_SECONDS, ParsedTokenTransfer, createMint, createTokenAccount, getPDAs } from "./utils";
// import { BN } from "bn.js";
describe("Initialize", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const { connection } = provider;
    const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

    it('correctly initializes a new account and transfers the funds', async () => {
        const employer = provider.wallet.publicKey;
        const employee = Keypair.generate().publicKey;
        const mint = await createMint(provider);
        const employerAccount = await createTokenAccount(provider, provider.wallet.publicKey, mint, 100_000 * LAMPORTS_PER_SOL);

        let params = makeParams('2020-01-01', 6, 4, ONE_DAY_IN_SECONDS, '10');
        // params.grantTokenAmount = new anchor.BN(0);

        const initializeTransaction = await program.methods
            .initialize(params)
            .accounts({
                employee,
                employer,
                mint,
                employerAccount,
            })
            .rpc(COMMITMENT);
        console.log(`[Initialize] ${initializeTransaction}`);

        const tx = await connection.getParsedTransaction(
          initializeTransaction,
          COMMITMENT,
        );
        
        // Ensure that inner transfer succeded.
        const { grant, escrowTokenAccount } = await getPDAs({
          employer,
          employee,
          programId: program.programId,
        });
        const transferIx: any = tx.meta.innerInstructions[0].instructions.find(
          ix => (ix as any).parsed.type === "transfer" && ix.programId.toBase58() == spl.TOKEN_PROGRAM_ID.toBase58()
        );
        const parsedInfo: ParsedTokenTransfer = transferIx.parsed.info;
        expect(parsedInfo).eql({
            amount: params.grantTokenAmount.toString(),
            authority: employer.toBase58(),
            destination: escrowTokenAccount.toBase58(),
            source: employerAccount.toBase58()
        });

        // Check data
        const grantData = await program.account.grant.fetch(grant);
        expect(grantData.employer.toBase58()).to.eq(employer.toBase58());
        expect(grantData.initialized).to.eq(true);
        expect(grantData.revoked).to.eq(false);
        expect(grantData.alreadyIssuedTokenAmount.toNumber()).to.eq(0);
        expect(grantData.params.cliffSeconds.toNumber()).to.eq(params.cliffSeconds.toNumber());
        expect(grantData.params.startUnix.toNumber()).to.eq(params.startUnix.toNumber());
        expect(grantData.params.durationSeconds.toNumber()).to.eq(params.durationSeconds.toNumber());
        expect(grantData.params.grantTokenAmount.toNumber()).to.eq(params.grantTokenAmount.toNumber());
        expect(grantData.params.secondsPerSlice.toNumber()).to.eq(params.secondsPerSlice.toNumber());
        expect(grantData.mint.toBase58()).to.eql(mint.toBase58())
        expect(grantData.employer.toBase58()).to.eql(employer.toBase58())
        expect(grantData.employee.toBase58()).to.eql(employee.toBase58())
        expect(grantData.bumps.grant).to.not.eql(0);
        expect(grantData.bumps.escrowAuthority).to.not.eql(0);
        expect(grantData.bumps.escrowTokenAccount).to.not.eql(0);
    });
});