Authentication-Results: myt6-0c6ff95e6b5b.qloud-c.yandex.net; spf=pass (myt6-0c6ff95e6b5b.qloud-c.yandex.net: domain of outlook.com designates 40.92.58.101 as permitted sender, rule=[ip4:40.92.0.0/15]) smtp.mail=alice@outlook.com; dkim=pass header.i=@outlook.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@yandex.ru>
