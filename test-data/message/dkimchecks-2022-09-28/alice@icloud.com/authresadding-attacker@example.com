Authentication-Results: bimi.icloud.com; bimi=skipped reason="insufficient dmarc"
Authentication-Results: dmarc.icloud.com; dmarc=none header.from=delta.blinzeln.de
Authentication-Results: dkim-verifier.icloud.com; dkim=none
Authentication-Results: spf.icloud.com; spf=none (spf.icloud.com: alice@delta.blinzeln.de does not designate permitted sender hosts) smtp.mailfrom=alice@delta.blinzeln.de
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
