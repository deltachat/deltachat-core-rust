Authentication-Results: mxs.mail.ru; spf=none () smtp.mailfrom=alice@delta.blinzeln.de smtp.helo=nx170.node02.secure-mailgate.com
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
