import * as anchor from '@coral-xyz/anchor';
import { Program, web3, AnchorError } from '@coral-xyz/anchor';
import * as token from '@solana/spl-token';
import { Amm } from '../target/types/amm';
import { expect } from 'chai';
import { readLogs } from './helper';
import { BN } from 'bn.js';
import { sqrt } from 'bn-sqrt';

interface Pool {
    auth: web3.Keypair;
    payer: web3.Keypair;
    mint0: web3.PublicKey;
    mint1: web3.PublicKey;
    vault0: web3.PublicKey;
    vault1: web3.PublicKey;
    vaultLP: web3.PublicKey;
    poolMint: web3.PublicKey;
    poolState: web3.PublicKey;
    poolAuthority: web3.PublicKey;
}

interface LPProvider {
    signer: web3.Keypair;
    userAta0: web3.PublicKey;
    userAta1: web3.PublicKey;
    lpAta: web3.PublicKey;
}

describe('Amm', () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    const connection = provider.connection;
    anchor.setProvider(provider);
    const wallet = provider.wallet;

    const program = anchor.workspace.amm as Program<Amm>;

    const [configPDA] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from(anchor.utils.bytes.utf8.encode('config'))],
        program.programId
    );
    let n_decimals = 9;
    let pool: Pool;

    it('Initialize', async () => {
        const tx = await program.methods
            .initialize(provider.wallet.publicKey, new BN(300))
            .rpc();

        const config = await program.account.config.fetch(configPDA);

        expect(config.owner.toBase58()).to.eq(wallet.publicKey.toBase58());
        expect(config.feeTo.toBase58()).to.eq(wallet.publicKey.toBase58());
        expect(config.fee.eq(new BN(300))).to.be.true;
    });

    it('SetFeeTo', async () => {
        const newFeeTo = web3.Keypair.generate();

        const tx = await program.methods
            .setFeeTo(newFeeTo.publicKey)
            .accounts({ config: configPDA })
            .rpc();

        const config = await program.account.config.fetch(configPDA);
        expect(config.feeTo.toBase58()).to.eq(newFeeTo.publicKey.toBase58());
    });

    it('SetFee', async () => {
        try {
            await program.methods
                .setFee(new BN(10000))
                .accounts({ config: configPDA })
                .rpc();
        } catch (e) {
            expect(e).to.be.instanceOf(AnchorError);
            expect((e as AnchorError).error.errorCode.code).to.eq('InvalidFee');
        }

        const tx = await program.methods
            .setFee(new BN(500))
            .accounts({ config: configPDA })
            .rpc();

        const config = await program.account.config.fetch(configPDA);
        expect(config.fee.eq(new BN(500))).to.be.true;
    });

    it('CreatePool', async () => {
        let auth = web3.Keypair.generate();
        let sig = await connection.requestAirdrop(
            auth.publicKey,
            100 * web3.LAMPORTS_PER_SOL
        );
        await connection.confirmTransaction(sig, 'confirmed');

        let mint0 = await token.createMint(
            connection,
            auth,
            auth.publicKey,
            auth.publicKey,
            n_decimals
        );
        let mint1 = await token.createMint(
            connection,
            auth,
            auth.publicKey,
            auth.publicKey,
            n_decimals
        );

        if (mint0.toBase58() > mint1.toBase58()) {
            [mint0, mint1] = [mint1, mint0];
        }

        let [poolState, poolState_b] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from('pool'), mint0.toBuffer(), mint1.toBuffer()],
            program.programId
        );

        let [poolAuthority, poolAuthority_b] =
            web3.PublicKey.findProgramAddressSync(
                [Buffer.from('authority'), poolState.toBuffer()],
                program.programId
            );

        let [poolMint, poolMint_b] = web3.PublicKey.findProgramAddressSync(
            [Buffer.from('lp_mint'), poolState.toBuffer()],
            program.programId
        );

        let vault0 = await token.getOrCreateAssociatedTokenAccount(
            connection,
            auth,
            mint0,
            poolAuthority,
            true
        );

        let vault1Address = token.getAssociatedTokenAddressSync(
            mint1,
            poolAuthority,
            true
        );

        try {
            await program.methods
                .createPool()
                .accounts({
                    owner: wallet.publicKey,
                    mint0: mint0,
                    mint1: mint1,
                    vault0: vault0.address,
                    vault1: vault1Address,
                })
                .rpc();
        } catch (e) {
            expect(e).to.be.instanceOf(AnchorError);
            expect((e as AnchorError).error.errorCode.number).to.eq(3012);
        }

        let vault1 = await token.getOrCreateAssociatedTokenAccount(
            connection,
            auth,
            mint1,
            poolAuthority,
            true
        );

        const tx = await program.methods
            .createPool()
            .accounts({
                owner: wallet.publicKey,
                mint0: mint0,
                mint1: mint1,
                vault0: vault0.address,
                vault1: vault1.address,
            })
            .rpc();

        const poolData = await program.account.pool.fetch(poolState);

        expect(poolData.token0.toBase58()).to.eq(mint0.toBase58());
        expect(poolData.token1.toBase58()).to.eq(mint1.toBase58());
        expect(poolData.reserve0.eq(new BN(0))).to.be.true;
        expect(poolData.reserve1.eq(new BN(0))).to.be.true;
        expect(poolData.kLast.eq(new BN(0))).to.be.true;

        let vaultLp = await token.getOrCreateAssociatedTokenAccount(
            connection,
            auth,
            poolMint,
            poolAuthority,
            true
        );

        pool = {
            auth,
            payer: auth,
            mint0,
            mint1,
            vault0: vault0.address,
            vault1: vault1.address,
            vaultLP: vaultLp.address,
            poolMint,
            poolState,
            poolAuthority,
        };
    });

    let lpUser0: LPProvider;
    let liquidityAdded: anchor.BN;
    let src_amount0_in = lp_amount(100);
    let src_amount1_in = lp_amount(100);
    it('AddLiquidity', async () => {
        let lp_user_signer = web3.Keypair.generate();
        let [userAta0, userAta1, lpAta] = await setup_lp_provider(
            lp_user_signer.publicKey,
            100
        );

        lpUser0 = {
            signer: lp_user_signer,
            userAta0,
            userAta1,
            lpAta,
        };

        await program.methods
            .addLiquidity(
                src_amount0_in,
                src_amount1_in,
                src_amount0_in,
                src_amount1_in
            )
            .accounts({
                owner: lp_user_signer.publicKey,
                pool: pool.poolState,
                vault0: pool.vault0,
                vault1: pool.vault1,
                vaultLp: pool.vaultLP,
                userAta0: lpUser0.userAta0,
                userAta1: lpUser0.userAta1,
                userLpAta: lpUser0.lpAta,
            })
            .signers([lpUser0.signer])
            .rpc();

        let poolData = await program.account.pool.fetch(pool.poolState);
        expect(poolData.reserve0.eq(src_amount0_in)).to.be.true;
        expect(poolData.reserve1.eq(src_amount1_in)).to.be.true;
        expect(poolData.kLast.eq(src_amount0_in.mul(src_amount1_in))).to.be
            .true;

        let userMint0Balance = await connection.getTokenAccountBalance(
            userAta0
        );
        let userMint1Balance = await connection.getTokenAccountBalance(
            userAta1
        );
        let userLpBalance = await connection.getTokenAccountBalance(lpAta);

        expect(userMint0Balance.value.amount).to.be.eq(
            lp_amount(100).sub(src_amount0_in).toString()
        );
        expect(userMint1Balance.value.amount).to.be.eq(
            lp_amount(100).sub(src_amount1_in).toString()
        );
        liquidityAdded = sqrt(src_amount0_in.mul(src_amount1_in));
        expect(userLpBalance.value.amount).to.be.eq(liquidityAdded.toString());

        let poolMint0Balance = await connection.getTokenAccountBalance(
            pool.vault0
        );
        let poolMint1Balance = await connection.getTokenAccountBalance(
            pool.vault1
        );
        expect(poolMint0Balance.value.amount).to.be.eq(
            src_amount0_in.toString()
        );
        expect(poolMint1Balance.value.amount).to.be.eq(
            src_amount1_in.toString()
        );

        let poolLp = await connection.getTokenSupply(pool.poolMint);
        expect(poolLp.value.amount).to.be.eq(userLpBalance.value.amount);
    });

    it('RemoveLiquidity', async () => {
        let userMint0BalanceBefore = await connection.getTokenAccountBalance(
            lpUser0.userAta0
        );
        let userMint1BalanceBefore = await connection.getTokenAccountBalance(
            lpUser0.userAta1
        );
        let userLpBalanceBefore = await connection.getTokenAccountBalance(
            lpUser0.lpAta
        );
        await program.methods
            .removeLiquidity(
                liquidityAdded.div(new anchor.BN(2)),
                src_amount0_in.div(new anchor.BN(2)),
                src_amount1_in.div(new anchor.BN(2))
            )
            .accounts({
                owner: lpUser0.signer.publicKey,
                pool: pool.poolState,
                vault0: pool.vault0,
                vault1: pool.vault1,
                vaultLp: pool.vaultLP,
                userAta0: lpUser0.userAta0,
                userAta1: lpUser0.userAta1,
                userLpAta: lpUser0.lpAta,
            })
            .signers([lpUser0.signer])
            .rpc();

        let userMint0BalanceAfter = await connection.getTokenAccountBalance(
            lpUser0.userAta0
        );
        let userMint1BalanceAfter = await connection.getTokenAccountBalance(
            lpUser0.userAta1
        );
        let userLpBalanceAfter = await connection.getTokenAccountBalance(
            lpUser0.lpAta
        );
        expect(
            new BN(userMint0BalanceAfter.value.amount)
                .sub(new BN(userMint0BalanceBefore.value.amount))
                .eq(src_amount0_in.div(new anchor.BN(2)))
        ).to.be.true;
        expect(
            new BN(userMint1BalanceAfter.value.amount)
                .sub(new BN(userMint1BalanceBefore.value.amount))
                .eq(src_amount1_in.div(new anchor.BN(2)))
        ).to.be.true;
        expect(
            new BN(userLpBalanceBefore.value.amount)
                .sub(new BN(userLpBalanceAfter.value.amount))
                .eq(liquidityAdded.div(new anchor.BN(2)))
        ).to.be.true;
    });

    async function setup_lp_provider(user: web3.PublicKey, amount: number) {
        // setup token accs for deposit
        let mint0_ata = await token.createAssociatedTokenAccount(
            connection,
            pool.payer,
            pool.mint0,
            user
        );
        let mint1_ata = await token.createAssociatedTokenAccount(
            connection,
            pool.payer,
            pool.mint1,
            user
        );

        // setup token accs for LP pool tokens
        let lp_mint_ata = await token.createAssociatedTokenAccount(
            connection,
            pool.payer,
            pool.poolMint,
            user
        );

        // setup initial balance of mints
        await token.mintTo(
            connection,
            pool.payer,
            pool.mint0,
            mint0_ata,
            pool.auth,
            amount * 10 ** n_decimals
        );
        await token.mintTo(
            connection,
            pool.payer,
            pool.mint1,
            mint1_ata,
            pool.auth,
            amount * 10 ** n_decimals
        );

        return [mint0_ata, mint1_ata, lp_mint_ata];
    }

    function lp_amount(n) {
        return new anchor.BN(n * 10 ** n_decimals);
    }
});
