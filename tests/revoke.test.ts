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

describe("Revoke", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const { connection } = provider;
    const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

    it('correctly marks an account as revoked and pays/refunds employer and employee', async () => {
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

        const threeYearsAgo = moment().subtract(2, 'years');
        const params = makeParams(threeYearsAgo, 12, 4, 1, '40000');
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
            .revoke()
            .accounts({
                employee,
                employer,
                grant,
                escrowAuthority,
                escrowTokenAccount,
                employeeAccount,
                employerAccount,
            })
            .rpc(COMMITMENT);
        const tx = await connection.getParsedTransaction(
          withdrawTransaction,
          COMMITMENT
        );
        expect(tx.meta.innerInstructions[0].instructions[0].programId.toBase58()).eql(spl.TOKEN_PROGRAM_ID.toBase58())
        expect((tx.meta.innerInstructions[0].instructions[1] as any).parsed.type).eql("transfer");
        const transferToEmployee: ParsedTokenTransfer = (tx.meta.innerInstructions[0].instructions[0] as any).parsed.info
        const transferToEmployer: ParsedTokenTransfer = (tx.meta.innerInstructions[0].instructions[1] as any).parsed.info
        expect(transferToEmployee.source).to.eq(escrowTokenAccount.toBase58())
        expect(transferToEmployee.destination).to.eq(employeeAccount.toBase58())
        expect(transferToEmployee.authority).to.eq(escrowAuthority.toBase58())
        expect(transferToEmployer.source).to.eq(escrowTokenAccount.toBase58())
        expect(transferToEmployer.destination).to.eq(employerAccount.toBase58())
        expect(transferToEmployer.authority).to.eq(escrowAuthority.toBase58())
        expect(parseInt(transferToEmployee.amount) + parseInt(transferToEmployer.amount)).to.eql(params.grantTokenAmount.toNumber());
        
        // Check data
        const grantData = await program.account.grant.fetch(grant, 'confirmed');
        expect(grantData.alreadyIssuedTokenAmount.toString()).to.eq(transferToEmployee.amount);
        expect(grantData.revoked).to.eq(true);

    });
});