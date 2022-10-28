Authentication-Results: mx.zohomail.eu;
From: forged-authres-added@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
