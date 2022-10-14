Authentication-Results: mxpostfix01.mail.de; dkim=none; dkim-atps=neutral
From: authresadding-attacker@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
