require 'test_helper'

class PageParserTest < ActiveSupport::TestCase

  def test_1
    filename = 'pages/forteresse_alamuth/http___lesitedontvousetesleheros_overblog_com_107_30'
    result = GameCore::PageParser.new.parse_page( File.open( filename ).read )
    pp result
  end

  def test_adjustement
    filename = 'pages/forteresse_alamuth/http___lesitedontvousetesleheros_overblog_com_595_0'
    result = GameCore::PageParser.new.parse_page( File.open( filename ).read )

    pp result

    test_result = {:page_elements=>
                     [{:type=>:text,
                       :text=>
                         "Vous plongez, le poignard à la main, prêt à affronter le monstre. Le choc est rude : la pieuvre était aussi préparée que vous ! Mais vous connaissez son point faible : son oeil unique. Vous le transpercerez, tuant ainsi la bête sur le coup, si, au cours du combat, vous réussissez un double. A cause de l'eau qui vous entoure, vous mènerez ce combat en retirant 2 points de votre total de Force du moment. Pour chaque 6 que vous réussirez, ôtez 2 points de Force à la pieuvre : vous aurez tranché l'un de ses tentacules. Cependant si vous ne remportez pas ce duel en quatre assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé. Le poulpe vous dévorera."},
                      {:type=>:text, :form=>:strong, :text=>"PIEUVRE VENIMEUSE "},
                      {:type=>:text, :text=>"FORCE : 16 VIE : 6 AJUSTEMENT DEGATS : + 1"},
                      {:type=>:a,
                       :href=>"http://lesitedontvousetesleheros.overblog.com/243-5",
                       :text=>"Si vous triomphez."}],
                   :monsters=>
                     [{:force=>"16", :vie=>"6", :adjustment=>1, :name=>"PIEUVRE VENIMEUSE "}]}

    assert_equal test_result, result
  end

end