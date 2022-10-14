ARC-Authentication-Results: i=1; mx3.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=smtpng1.i.mail.ru policy.ptr=smtpng1.i.mail.ru;
    bimi=none (No BIMI records found);
    arc=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=mail.ru header.i=@mail.ru
    header.b=0EDw+VrK header.a=rsa-sha256 header.s=mail4 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=reject,d=none,d.eval=none) policy.policy-from=p
    header.from=mail.ru;
    iprev=pass smtp.remote-ip=94.100.181.251 (smtpng1.i.mail.ru);
    spf=pass smtp.mailfrom=alice@mail.ru smtp.helo=smtpng1.i.mail.ru
Authentication-Results: mx3.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=smtpng1.i.mail.ru policy.ptr=smtpng1.i.mail.ru
Authentication-Results: mx3.messagingengine.com;
    bimi=none (No BIMI records found)
Authentication-Results: mx3.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx3.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=mail.ru header.i=@mail.ru
      header.b=0EDw+VrK header.a=rsa-sha256 header.s=mail4 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=reject,d=none,d.eval=none) policy.policy-from=p
      header.from=mail.ru;
    iprev=pass smtp.remote-ip=94.100.181.251 (smtpng1.i.mail.ru);
    spf=pass smtp.mailfrom=alice@mail.ru smtp.helo=smtpng1.i.mail.ru
From: <alice@mail.ru>
To: <alice@fastmail.com>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
