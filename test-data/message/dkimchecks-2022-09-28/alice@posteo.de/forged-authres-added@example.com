Authentication-Results: posteo.de; dmarc=none (p=none dis=none) header.from=delta.blinzeln.de
Authentication-Results: posteo.de; spf=tempfail smtp.mailfrom=delta.blinzeln.de
From: forged-authres-added@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
Authentication-Results: aaa.com; dkim=pass header.i=@example.com
