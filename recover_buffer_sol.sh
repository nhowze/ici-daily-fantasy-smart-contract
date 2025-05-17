#!/usr/bin/expect -f

set seeds {
    "pitch sand verb leg hill easily judge orchard similar garment forget group"
    "face dutch shaft ceiling devote hope report element travel universe access hunt"
    "essence system near govern know only clog gadget chunk slight spoon tortoise"
    "wisdom cool swim bright chuckle make pride version card deposit subway flight"
    "detail month level mom style grief tuna across brass dad kit bronze"
    "firm salt inch normal use cactus seminar beach bonus work december hollow"
}

set recipient "HC7VdyQH11fkCP3LpHDvmXYMM1uWN4CkXN9EoZpEAcmf"
set cluster "https://api.devnet.solana.com"

for {set i 0} {$i < [llength $seeds]} {incr i} {
    set seed [lindex $seeds $i]
    set outfile "/tmp/buffer_keypair_$i.json"

    puts "🔁 Recovering keypair $i..."

    spawn solana-keygen recover prompt://bip39 --outfile $outfile --force
    expect "Enter seed phrase:"
    send "$seed\r"
    expect "If this seed phrase has an associated passphrase"
    send "\r"
    expect "Continue? (y/n):"
    send "y\r"
    expect eof

    # Wait for the file to be created
    set attempts 0
    while {![file exists $outfile] && $attempts < 10} {
        exec sleep 1
        incr attempts
    }

    if {![file exists $outfile]} {
        puts "❌ Keypair file not found at $outfile — skipping..."
        continue
    }

    set pubkey [exec solana-keygen pubkey $outfile]
    puts "➡️ Attempting to close buffer account $pubkey..."

    catch {
        exec solana program close $pubkey --recipient $recipient --url $cluster
    } result

    puts $result
    puts ""
}

puts "✅ Done"
