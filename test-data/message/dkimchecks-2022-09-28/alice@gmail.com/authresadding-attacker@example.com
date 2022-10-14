Authentication-Results: mx.google.com;
       spf=neutral (google.com: 89.22.108.212 is neither permitted nor denied by best guess record for domain of alice@delta.blinzeln.de) smtp.mailfrom=alice@delta.blinzeln.de
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
